use std::path::PathBuf;

use anyhow::{Context, Result};
use colored::Colorize;

use intern::GetStr;
use workflow::{ModuleId, RunStrId, Workflow};

use crate::fs::Fs;

use super::{TaskRunner, TaskRunnerBuilder};

struct DeleteAction {
    realization: RunStrId,
    print: RunStrId,
}

/// Cleans up old run dirs and creates new ones in preparation for executing the traversal.
pub struct PreRunner<'a> {
    /// for filesystem operations
    fs: &'a Fs,
    /// workflow information
    wf: &'a Workflow,
    /// print out more ui messages
    verbose: bool,
}

impl<'a> PreRunner<'a> {
    pub fn new(fs: &'a Fs, wf: &'a Workflow, verbose: bool) -> Self {
        Self { fs, wf, verbose }
    }

    /// print list of tasks in a traversal that are:
    /// - already complete
    /// - to be deleted, directories recreated, and re-run
    /// - new, directories will be created and run for the first time
    /// - if verbose, will also print out modules used.
    pub fn print_actions(&self, actions: &Actions) {
        if !actions.completed.is_empty() {
            eprintln!(
                "\nThe following tasks are {} and will not run:",
                "already complete".green()
            );
            for id in &actions.completed {
                eprintln!("{} {}", "COMPLETED".green(), self.wf.strings.run.get(*id));
            }
        }

        if !actions.to_delete.is_empty() {
            eprintln!(
                "\nThe following tasks are {} and will be deleted:",
                "incomplete or invalid".red()
            );
            for to_delete in &actions.to_delete {
                eprintln!(
                    "{} {}",
                    "DELETE".red(),
                    self.wf.strings.run.get(to_delete.print)
                );
            }
        }

        if !actions.to_run.is_empty() {
            eprintln!("\nThe following tasks {}:", "will run".green());
            for runner in &actions.to_run {
                eprintln!(
                    "{} {}",
                    "RUN".green(),
                    self.wf.strings.run.get(runner.print_id)
                );
            }
        }

        if self.verbose && !actions.modules.is_empty() {
            eprintln!("\nThe following {} will be used: ", "modules".magenta());
            for module in &actions.modules {
                eprintln!(
                    "{}: {}",
                    self.wf.strings.modules.get(*module).magenta(),
                    self.wf.get_module_path(*module),
                );
            }
        }

        eprintln!();
    }

    /// actually clean up and prepare the output directory for running the workflow.
    pub fn do_pre_run_actions(&mut self, actions: Actions) -> Result<Vec<TaskRunner>> {
        self.do_delete(&actions)?;
        self.prep_and_convert_to_runners(actions)
    }

    fn do_delete(&self, actions: &Actions) -> Result<()> {
        // In the future when we invalidate a task and its antecedents,
        // we'd like to leave a log line in a text file so we can audit over multiple runs.
        for to_delete in &actions.to_delete {
            let realization = self.wf.strings.run.get(to_delete.realization);
            eprintln!("{} {}", "Deleting".red(), realization);
            self.fs
                .delete_dir(realization)
                .with_context(|| format!("while deleting old realization {}", realization))?;
        }
        Ok(())
    }

    fn prep_and_convert_to_runners(&mut self, actions: Actions) -> Result<Vec<TaskRunner>> {
        let mut runners = Vec::with_capacity(actions.to_run.len());
        let mut task_sh_contents = String::with_capacity(1024);
        let mut task_sh_path = PathBuf::with_capacity(128);

        for builder in actions.to_run {
            let realization = self.wf.strings.run.get(builder.realization_id);

            eprintln!("{} {}", "Creating".green(), realization);
            self.fs
                .create_dir(realization)
                .context("creating realization dir")?;

            let symlink = self.wf.strings.run.get(builder.symlink_id);
            let link_target = self.wf.strings.run.get(builder.link_target_id);

            if self.verbose {
                eprintln!("{} {} to {}", "Symlinking".magenta(), symlink, link_target);
            }
            if self.fs.exists(symlink) {
                log::info!("symlink {} already exists; deleting", symlink);
                self.fs.delete_file(symlink)?;
            }
            self.fs.symlink(link_target, symlink)?;

            // NB this puts the contents of task.sh into self.strbuf:
            let runner =
                builder.into_task_runner(&self.wf.strings.run, self.wf, &mut task_sh_contents);

            if self.verbose {
                eprintln!("{}", "Writing task.sh file.".magenta());
            }
            let task_sh = self.fs.task_sh(realization, &mut task_sh_path);
            self.fs
                .write_file(task_sh, &task_sh_contents)
                .context("writing task.sh file")?;

            runners.push(runner);
        }
        Ok(runners)
    }
}

/// Contains the information needed to prepare the workflow directory for a run.
pub struct Actions {
    completed: Vec<RunStrId>,
    to_delete: Vec<DeleteAction>,
    to_run: Vec<TaskRunnerBuilder>,
    modules: Vec<ModuleId>,
}

impl Actions {
    pub fn new(len: usize) -> Self {
        Self {
            completed: Vec::with_capacity(len),
            to_delete: Vec::with_capacity(len),
            to_run: Vec::with_capacity(len),
            modules: Vec::with_capacity(4),
        }
    }

    pub fn has_tasks_to_run(&self) -> bool {
        !self.to_run.is_empty()
    }

    pub fn add_delete(&mut self, print_id: RunStrId, realization_id: RunStrId) {
        self.to_delete.push(DeleteAction {
            realization: realization_id,
            print: print_id,
        });
    }

    pub fn add_completed(&mut self, print_id: RunStrId) {
        self.completed.push(print_id);
    }

    pub fn add_run(&mut self, action: TaskRunnerBuilder) {
        self.to_run.push(action);
    }

    pub fn modules_mut(&mut self) -> &mut Vec<ModuleId> {
        &mut self.modules
    }
}
