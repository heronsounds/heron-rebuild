use std::path::{Path, PathBuf};

use anyhow::Result;
use colored::Colorize;

use intern::InternStr;
use util::PathEncodingError;
use workflow::{parse_compact_branch_str, BranchSpec, Workflow};

use crate::fs::Fs;
use crate::settings::Settings;
use crate::ui::Ui;

/// Logic for invalidating tasks from previous executions.
pub struct Invalidator<'a> {
    fs: &'a Fs,
    ui: &'a Ui,
    settings: &'a Settings,
}

impl<'a> Invalidator<'a> {
    /// Create a new `Invalidator`.
    pub fn new(settings: &'a Settings, ui: &'a Ui, fs: &'a Fs) -> Self {
        Self { settings, ui, fs }
    }
}

impl Invalidator<'_> {
    /// Invalidate tasks from `wf`, using the targets defined in settings.
    pub fn invalidate(&self, wf: &mut Workflow) -> Result<()> {
        use crate::settings::ArgsBranch;

        if self.settings.tasks.is_empty() {
            eprintln!("No tasks specified; quitting.");
        }

        let mut pathbuf = PathBuf::with_capacity(256);
        match &self.settings.branches {
            // if no branch specified, we delete entire task realizations dirs:
            ArgsBranch::Empty => {
                for task in &self.settings.tasks {
                    eprintln!(
                        "{} of task {}.",
                        "No branch specified; invalidating all realizations".magenta(),
                        task.cyan()
                    );
                    let realizations = self.fs.realizations_dir(task, &mut pathbuf);
                    self.delete_dir_if_exists(realizations)?;
                }
            }
            // if branch is literally "Baseline.baseline", we delete that branch for each task:
            ArgsBranch::Baseline => {
                for task in &self.settings.tasks {
                    eprintln!(
                        "{} of task {}.",
                        "Invalidating baseline realization".magenta(),
                        task.cyan()
                    );
                    let realization = self.fs.baseline_realization(task, &mut pathbuf);
                    self.delete_dir_if_exists(realization)?;
                }
            }
            // o/w, branch was specified, so we look for matching branches in each task:
            ArgsBranch::Specified(strs) => {
                let mut arg_branch = BranchSpec::default();
                for (k, v) in strs {
                    let k = wf.strings.branchpoints.intern(k);
                    let v = wf.strings.idents.intern(v);
                    arg_branch.insert(k, v);
                }
                for task in &self.settings.tasks {
                    if self.settings.verbose {
                        eprintln!(
                            "{} in task {}.",
                            "Searching for realizations to invalidate".magenta(),
                            task.cyan()
                        );
                    }
                    self.invalidate_task_branch(task, wf, &arg_branch, &mut pathbuf)?;
                }
            }
        }
        Ok(())
    }

    fn invalidate_task_branch(
        &self,
        task: &str,
        wf: &mut Workflow,
        arg_branch: &BranchSpec,
        pathbuf: &mut PathBuf,
    ) -> Result<()> {
        let realizations = self.fs.realizations_dir(task, pathbuf);
        let mut found_any = false;
        if self.fs.is_dir(realizations)? {
            for entry in self.fs.read_dir(self.fs.realizations_dir(task, pathbuf))? {
                let entry = entry?;
                let matches = {
                    let fpath = entry.file_name();
                    let fname = fpath.to_str().ok_or(PathEncodingError)?;
                    let entry_branch = parse_compact_branch_str(wf, fname)?;
                    arg_branch.is_exact_match(&entry_branch)
                };
                if matches {
                    eprintln!("{} {:?}", "Invalidating".magenta(), entry.path());
                    found_any = true;
                    let exit_code = self.fs.exit_code(&entry.path(), pathbuf);
                    if self.fs.exists(exit_code) {
                        eprintln!("{} {exit_code:?}", "Deleting".red());
                        if !self.settings.dry_run && self.ui.confirm("Proceed?")? {
                            self.fs.delete_file(exit_code)?;
                        }
                    } else {
                        eprintln!("Task is already invalid; not deleting.");
                    }
                }
            }
        }
        if !found_any {
            eprintln!("No matching realizations to invalidate.");
        }
        Ok(())
    }

    fn delete_dir_if_exists(&self, path: &Path) -> Result<()> {
        eprintln!("{} {path:?}.", "Deleting".red());
        if self.settings.dry_run || !self.ui.confirm("Proceed?")? {
            return Ok(());
        } else if self.fs.is_dir(path)? {
            self.fs.delete_dir(path)?;
        } else {
            eprintln!("{path:?} does not exist; not deleting.");
        }
        Ok(())
    }
}
