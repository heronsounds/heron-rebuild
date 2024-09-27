use std::path::PathBuf;

use anyhow::{Context, Result};
use colored::Colorize;

use intern::{GetStr, TypedStrs};
use workflow::RunStrId;

use crate::fs::Fs;
use crate::prep::TaskRunner;
use crate::ui::Ui;

use super::{run_cmd::run_cmd, Error};

/// `WorkflowRunner` is the struct that actually runs a workflow.
///
/// It traverses the list of tasks provided by the `prep` module one final time,
/// ensures that all input files exist, then executes the task code.
/// After execution, it confirms that all output files exist before moving to
/// the next task. In the case of module tasks, it will copy output files from
/// the module directory back into the task directory so that dependent tasks
/// can find them. When a task is complete, it writes an `exit_code` file to
/// the task directory so that subsequent runs will not try to execute the
/// task again.
pub struct WorkflowRunner {
    /// interned strings containing all file paths used by this execution run
    run_strs: TypedStrs<RunStrId>,
    /// for whenever we need to create a path:
    pathbuf: PathBuf,
    /// Filesystem interface
    fs: Fs,
    /// User interface
    ui: Ui,
}

impl WorkflowRunner {
    /// Create a new `WorkflowRunner`.
    pub fn new(run_strs: TypedStrs<RunStrId>, fs: Fs, ui: Ui) -> Self {
        Self {
            run_strs,
            pathbuf: PathBuf::with_capacity(256),
            fs,
            ui,
        }
    }

    pub fn run(&mut self, mut tasks: Vec<TaskRunner>) -> Result<()> {
        debug_assert!(!tasks.is_empty());

        for task in &mut tasks {
            self.ui.start_timer();
            let realization_dir = self.run_strs.get(task.realization_dir);
            let task_str = self.run_strs.get(task.print_id);
            eprintln!("{} {task_str}\nin {realization_dir}\n", "RUN".green());

            if self.ui.verbose {
                eprintln!("\n{}", "Checking that all inputs exist...".magenta());
            }
            self.check_files_exist(&task.inputs)
                .context("while checking for input files")?;
            if self.ui.verbose {
                eprintln!("All input files were found.\n");
            }

            let success = run_cmd(
                &mut task.cmd,
                realization_dir,
                &mut self.fs,
                &mut self.pathbuf,
                self.ui.verbose,
            )?;
            if !success {
                return Err(Error::SubprocessFailed.into());
            }

            if !task.copy_outputs_to.is_empty() {
                if self.ui.verbose {
                    eprintln!(
                        "\n{}\n",
                        "Copying outputs from module back to task dir...".magenta()
                    );
                }
                self.copy_module_outputs(task, &self.fs)
                    .context("while copying module outputs to realization dir")?;
                if self.ui.verbose {
                    eprintln!("All module outputs copied.");
                }
            } else {
                if self.ui.verbose {
                    eprintln!(
                        "\n{}",
                        "Checking that all expected outputs exist...".magenta()
                    );
                }
                self.check_files_exist(&task.outputs)
                    .context("while checking for output files")?;
                if self.ui.verbose {
                    eprintln!("All output files were found.");
                }
            }

            self.ui.print_elapsed("Task execution")?;

            eprintln!(
                "{} {task_str}. Writing exit_code file.\n",
                "COMPLETED".green()
            );
            let exit_code = self
                .fs
                .exit_code(realization_dir.as_ref(), &mut self.pathbuf);
            self.fs
                .write_file(exit_code, "0")
                .context("while writing exit_code file for successful task.")?;
        }
        eprintln!("{}\n", "Completed workflow.".green());

        Ok(())
    }

    fn copy_module_outputs(&self, task: &TaskRunner, fs: &Fs) -> Result<()> {
        for (id, file) in task.outputs.iter().enumerate() {
            let file = self.run_strs.get(*file);
            let copy_to_file = self.run_strs.get(task.copy_outputs_to[id]);

            self.check_file_exists(file)
                .context("while checking for output file in module")?;

            fs.create_parent_dir(copy_to_file)?;
            fs.copy(file, copy_to_file)?;
        }
        Ok(())
    }

    fn check_files_exist(&self, file_ids: &[RunStrId]) -> Result<(), Error> {
        for file in file_ids {
            self.check_file_exists(self.run_strs.get(*file))?;
        }
        Ok(())
    }

    fn check_file_exists(&self, file: &str) -> Result<(), Error> {
        if !self.fs.exists(file) {
            Err(Error::ExpectedFileNotFound(file.to_owned()))
        } else {
            if self.ui.verbose {
                eprintln!(" - {file}");
            }
            Ok(())
        }
    }
}
