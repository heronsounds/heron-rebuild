use std::path::PathBuf;

use anyhow::{Context, Result};
use colored::Colorize;

use intern::{GetStr, InternStr, TypedInterner};
use syntax::{self, ast};
use traverse::Traversal;
use workflow::{BranchMask, Workflow};

use crate::exec::WorkflowRunner;
use crate::fs::Fs;
use crate::invalidate::Invalidator;
use crate::prep::{PreRunner, TraversalResolver};
use crate::settings::{Action, Settings};
use crate::ui::Ui;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Workflow has more than the maximum number of branchpoints (8)")]
    TooManyBranchpoints,
}

/// This struct actually runs the command-line app.
pub struct App {
    settings: Settings,
    fs: Fs,
    ui: Ui,
}

impl App {
    /// Create a new `App`.
    pub fn new(settings: Settings) -> Self {
        let fs = Fs::new(&settings.output, settings.dry_run);
        let ui = Ui::new(&settings);
        Self { settings, fs, ui }
    }

    /// Run the app, using settings to determine which task to run.
    pub fn run(mut self) -> Result<()> {
        let mut pathbuf = PathBuf::with_capacity(512);
        let branchpoints_file = self.fs.branchpoints_txt(&mut pathbuf);
        let mut strbuf = String::with_capacity(0); // will be resized later.

        let mut wf = Workflow::default();
        self.fs
            .load_branchpoints_file(branchpoints_file, &mut wf, &mut strbuf, &self.ui)?;

        match &self.settings.action {
            Action::RunPlan(plan) => {
                if self.settings.verbose {
                    eprintln!("Using output directory \"{:?}\".", self.settings.output,);
                }
                self.fs.ensure_output_dir_exists(self.settings.verbose)?;

                let plan = wf.strings.idents.intern(plan);
                self.parse_workflow(&mut strbuf, &mut wf)?;

                // update branchpoints file (we may have added new branchpoints in this run):
                if !self.settings.dry_run {
                    self.fs
                        .write_branchpoints_file(branchpoints_file, &wf, &mut strbuf)?;
                }

                wf.strings.alloc_for_traversal();

                self.ui.verbose_progress("Creating traversal");
                let traversal = Traversal::create(&wf, plan, self.settings.verbose)?;
                self.ui.done();

                self.run_traversal(wf, traversal)?;
            }
            Action::Invalidate => {
                let invalidator = Invalidator::new(self.settings, self.ui, self.fs);
                invalidator.invalidate(&mut wf)?;
            }
        }

        Ok(())
    }
}

// PARSING //////////////////
impl App {
    fn parse_workflow(&mut self, strbuf: &mut String, builder: &mut Workflow) -> Result<()> {
        self.read_config_to_buf(strbuf)?;
        let blocks = self.parse_config(&*strbuf)?;

        self.ui.verbose_progress("Creating workflow");
        self.ui.start_timer();

        builder.load(blocks, self.settings.config_parent_dir()?)?;

        if builder.strings.branchpoints.len() > BranchMask::BITS as usize {
            return Err(Error::TooManyBranchpoints.into());
        }
        self.ui.done();
        self.ui.print_elapsed("Creating workflow")?;

        if self.settings.verbose {
            eprintln!(
                "Created workflow with {} tasks and {} branchpoints.",
                builder.strings.tasks.len(),
                builder.strings.branchpoints.len()
            );
        }

        Ok(())
    }

    fn read_config_to_buf(&mut self, strbuf: &mut String) -> Result<()> {
        self.ui
            .verbose_progress_debug("Reading config file", &self.settings.config);
        self.fs
            .read_to_buf(&self.settings.config, strbuf)
            .with_context(|| format!("while reading config file \"{:?}\"", self.settings.config))?;
        self.ui.done();
        Ok(())
    }

    fn parse_config<'a>(&mut self, text: &'a str) -> Result<Vec<ast::Item<'a>>> {
        self.ui.verbose_progress("Parsing config file");
        self.ui.start_timer();
        let blocks = syntax::parse(text)
            .with_context(|| format!("while parsing config file \"{:?}\"", self.settings.config))?;
        self.ui.done();
        self.ui.print_elapsed("Parsing config file")?;
        Ok(blocks)
    }
}

// RUNNING /////////////////
impl App {
    fn run_traversal(mut self, mut wf: Workflow, traversal: Traversal) -> Result<()> {
        // allocate space for run strs:
        wf.strings.alloc_for_run();

        // ensure no destructive operations on fs:
        self.fs.set_dry_run(true);

        // resolve traversal into completed/delete/run actions:
        let mut resolver = TraversalResolver::new(traversal.nodes.len(), &self.fs, &mut wf);
        let actions = resolver
            .resolve_to_actions(traversal)
            .context("while preparing tasks for workflow run")?;

        if !actions.has_tasks_to_run() {
            eprintln!("{}", "No tasks to run; exiting.".green());
            return Ok(());
        }

        // allow destructive fs operations again:
        self.fs.set_dry_run(false);

        // print summary of actions and confirm w/ user:
        let mut pre_runner = PreRunner::new(&self.fs, &wf, self.settings.verbose);
        pre_runner.print_actions(&actions);
        if self.settings.dry_run || !self.ui.confirm("Proceed?")? {
            return Ok(());
        }

        // delete old incomplete tasks and create new task dirs:
        let tasks = pre_runner
            .do_pre_run_actions(actions)
            .context("while preparing output directory for workflow run")?;

        eprintln!("\n{}.", "Workflow preparation complete".green());
        eprintln!("\n{}.\n", "Starting workflow execution".magenta());

        // actually run the tasks:
        let run_strs = TypedInterner::new(wf.strings.run.into_inner().into());
        let mut runner = WorkflowRunner::new(run_strs, self.fs, self.ui);
        runner.run(tasks).context("while running workflow")?;

        Ok(())
    }
}
