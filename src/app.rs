use std::path::PathBuf;

use anyhow::{Context, Result};
use colored::Colorize;

use intern::{GetStr, InternStr, TypedInterner};
use syntax::{self, ast};
use traverse::Traversal;
use workflow::{BranchSpec, Plan, Workflow};

use crate::exec::WorkflowRunner;
use crate::fs::Fs;
use crate::invalidate::Invalidator;
use crate::prep::{PreRunner, TraversalResolver};
use crate::settings::{ArgsBranch, Settings};
use crate::ui::Ui;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Nothing to run: no target specified with --plan or --task")]
    NoTargetSpecified,
    #[error("Multiple branches on command line are not yet supported")]
    MultiBranch,
    #[error("Too many branchpoints; maximum supported is 128")]
    TooManyBranchpoints,
}

/// This struct actually runs the command-line app.
pub struct App {
    /// Interpreted command line settings
    settings: Settings,
    /// Filesystem interface
    fs: Fs,
    /// User interface
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
        if self.settings.verbose > 0 {
            eprintln!("Using output directory {:?}", self.settings.output);
        }
        self.fs.ensure_out_dir_exists(self.settings.verbose > 0)?;

        let mut branch_file = PathBuf::with_capacity(512);
        self.fs.branchpoints_txt(&mut branch_file);

        let mut strbuf = String::with_capacity(0); // will be resized later.

        let mut wf = Workflow::default();

        // load branch file into wf first (if it exists),
        // so that branch ordering is consistent between runs:
        self.fs.load_branches(&branch_file, &mut wf, &mut strbuf, &self.ui)?;

        if self.settings.invalidate {
            let invalidator = Invalidator::new(&self.settings, &self.ui, &self.fs);
            invalidator.invalidate(&mut wf)?;
        }

        if self.settings.run {
            self.parse_workflow(&mut strbuf, &mut wf)?;

            if !self.settings.dry_run {
                log::info!("writing branchpoints.txt file");
                self.fs.write_branches(&branch_file, &wf, &mut strbuf)?;
            }

            let traversal = self.make_traversal(&mut wf)?;
            self.run_traversal(wf, traversal)?;
        }

        Ok(())
    }

    fn make_traversal(&self, wf: &mut Workflow) -> Result<Traversal> {
        let plan = self.get_target_for_run(wf)?;

        wf.strings.alloc_for_traversal();
        self.ui.verbose_progress("Creating traversal");
        let traversal = match wf.strings.branchpoints.len() {
            x if x <= 8 => Traversal::create::<u8>(wf, plan)?,
            x if x <= 16 => Traversal::create::<u16>(wf, plan)?,
            x if x <= 32 => Traversal::create::<u32>(wf, plan)?,
            x if x <= 64 => Traversal::create::<u64>(wf, plan)?,
            x if x <= 128 => Traversal::create::<u128>(wf, plan)?,
            _ => return Err(Error::TooManyBranchpoints.into()),
        };
        self.ui.done();

        log::debug!(
            "Traversal has {} inputs and {} outputs/params.",
            traversal.inputs.len(),
            traversal.outputs_params.len(),
        );

        Ok(traversal)
    }
}

// PARSING //////////////////
impl App {
    fn parse_workflow(&mut self, strbuf: &mut String, wf: &mut Workflow) -> Result<()> {
        self.read_config_to_buf(strbuf)?;
        let blocks = self.parse_config(&*strbuf)?;

        self.ui.verbose_progress("Creating workflow");
        self.ui.start_timer();

        wf.load(blocks, self.settings.config_parent_dir()?)?;

        self.ui.done();
        self.ui.print_elapsed("Creating workflow")?;

        if self.settings.verbose > 0 {
            eprintln!(
                "Created workflow with {} tasks and {} branchpoints.",
                wf.strings.tasks.len(),
                wf.strings.branchpoints.len()
            );
            wf.strings.log_sizes();
        }

        Ok(())
    }

    fn read_config_to_buf(&mut self, strbuf: &mut String) -> Result<()> {
        self.ui.verbose_progress_debug("Reading config file", &self.settings.config);
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
        let actions = resolver.resolve_to_actions(traversal)?;

        log::debug!(
            "{} Run strs, str len {}",
            wf.strings.run.len(),
            wf.strings.run.str_len()
        );

        if !actions.has_tasks_to_run() {
            eprintln!("{}", "No tasks to run; exiting.".green());
            return Ok(());
        }

        // allow destructive fs operations again:
        self.fs.set_dry_run(false);

        // print summary of actions and confirm w/ user:
        let mut pre_runner = PreRunner::new(&self.fs, &wf, self.settings.verbose > 0);
        pre_runner.print_actions(&actions)?;
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

// GETTING TARGETS ////////////
impl App {
    fn get_target_for_run(&self, wf: &mut Workflow) -> Result<Plan> {
        if let Some(plan_name) = &self.settings.plan {
            self.get_plan_target(plan_name, wf)
        } else if !self.settings.tasks.is_empty() {
            self.get_task_target(wf)
        } else {
            Err(Error::NoTargetSpecified.into())
        }
    }

    fn get_plan_target(&self, plan_name: &str, wf: &mut Workflow) -> Result<Plan> {
        log::debug!("Using plan {plan_name} specified on command line");
        let id = wf.strings.idents.intern(plan_name)?;
        Ok(wf.get_plan(id)?.clone())
    }

    fn get_task_target(&self, wf: &mut Workflow) -> Result<Plan> {
        log::debug!(
            "No plan specified; running tasks '{}' specified on command line",
            self.settings.tasks.join(", "),
        );
        let branch = self.get_target_branch(wf)?;
        Plan::create_anonymous(&mut wf.strings, &self.settings.tasks, branch)
    }

    fn get_target_branch(&self, wf: &mut Workflow) -> Result<BranchSpec> {
        let mut branch = BranchSpec::default();
        if let ArgsBranch::Specified(branch_pairs) = &self.settings.branches {
            for (k, v) in branch_pairs {
                let k = wf.strings.branchpoints.intern(k)?;
                if branch.is_specified(k) {
                    return Err(Error::MultiBranch.into());
                }
                let v = wf.strings.idents.intern(v)?;
                branch.insert(k, v);
            }
        }
        Ok(branch)
    }
}
