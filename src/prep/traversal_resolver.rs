use std::path::Path;

use anyhow::Result;

use intern::{GetStr, InternStr};
use traverse::{Node, RealInput, RealOutput, Traversal, ValueContext};
use util::PathEncodingError;
use workflow::{Errors, IdentId, RealTaskKey, Recapper, RunStrId, TaskVars, Workflow};

use crate::fs::Fs;

use super::{
    Actions, ActualTaskId, Deduper, Error, ModuleChecker, RealInputs, RealOutputsParams,
    TaskDirPaths, TaskRunnerBuilder, TaskVarChecker,
};

/// `TraversalResolver` turns Nodes into workflow actions to run.
/// Actions can be either to delete, or create and run.
/// Also returns a list of already completed tasks so they can be printed out to the user.
///
/// - Traverses the list of tasks to run
/// - Removes duplicates (we can have these if multiple fully-resolved branches turn out to be the same)
/// - Checks filesystem to see if tasks are already completed
/// - Prunes completed tasks
/// - Fully resolves all input files, output files, and params
pub struct TraversalResolver<'a> {
    /// keep track of vars for each task so we can check that they're defined:
    var_checker: TaskVarChecker,
    /// keep track of modules we've checked for existence already:
    module_checker: ModuleChecker,
    /// keep track of which tasks will actually run:
    should_run: Vec<bool>,
    /// store task outputs so that dependents can refer to them:
    outputs: Vec<Vec<(IdentId, RunStrId)>>,
    /// keep track of duplicate tasks:
    deduper: Deduper,
    /// interface to the filesystem:
    fs: &'a Fs,
    /// workflow definition
    wf: &'a mut Workflow,
    /// mainly used for fully-resolving interpolated string values
    strbuf: String,
    /// store errors here and display them at the end:
    errors: Errors,
}

impl<'a> TraversalResolver<'a> {
    pub fn new(len: usize, fs: &'a Fs, wf: &'a mut Workflow) -> Self {
        Self {
            var_checker: TaskVarChecker::with_capacity(wf.sizes().max_vars as usize),
            module_checker: ModuleChecker::with_capacity(wf.strings.modules.len()),
            outputs: Vec::with_capacity(len),
            should_run: Vec::with_capacity(len),
            deduper: Deduper::with_capacity(len),
            wf,
            fs,
            strbuf: String::with_capacity(256),
            errors: Errors::default(),
        }
    }
}

impl TraversalResolver<'_> {
    /// NB should only be run once; if we want multiple runs for some reason need a reset fn.
    pub fn resolve_to_actions(&mut self, traversal: Traversal) -> Result<Actions> {
        let mut paths = TaskDirPaths::new();
        let mut actions = Actions::new(traversal.nodes.len());
        for task in &traversal.nodes {
            if self.deduper.is_dupe(&task.key) {
                continue;
            }

            let should_run = self.resolve_to_action(
                task,
                &traversal.inputs,
                &traversal.outputs_params,
                &mut actions,
                &mut paths,
            )?;

            self.should_run.push(should_run);
        }

        self.errors.print_recap("preparing workflow", &self.wf.strings)?;
        Ok(actions)
    }

    /// returns true if task should run
    fn resolve_to_action(
        &mut self,
        task: &Node,
        inputs: &RealInputs,
        outputs_params: &RealOutputsParams,
        actions: &mut Actions,
        paths: &mut TaskDirPaths,
    ) -> Result<bool> {
        self.var_checker.clear();
        paths.make_paths(task, self.wf, self.fs, &mut self.strbuf)?;
        let mut vars = TaskVars::new_with_sizes(&task.vars);

        // handle inputs and outputs first, since we need those even if task won't run:
        let invalidated = self.handle_inputs(task, &mut vars.inputs, inputs)?;
        let copy_outputs_to =
            self.handle_outputs(task, &mut vars.outputs, outputs_params, paths)?;

        let real_task_string = self.wf.strings.get_real_task_str(&task.key)?.to_owned();
        let print_id = self.wf.strings.run.intern(real_task_string)?;
        let realization_id = self.make_path_id(paths.realization())?;

        // if task dir exists, check if it's complete; add to delete list if not:
        if self.fs.exists(paths.realization()) {
            if !invalidated && paths.exit_code_success(self.fs, &mut self.strbuf)? {
                actions.add_completed(print_id);
                return Ok(false);
            } else {
                actions.add_delete(print_id, realization_id);
            }
        }

        // at this point we know the task will run, so handle params:
        self.handle_params(task, &mut vars.params, outputs_params)?;

        // and perform some checks:
        let _ = self.var_checker.check(task, self.wf).map_err(|e| self.errors.add(e));
        let _ = self
            .module_checker
            .check(task, paths, self.fs, actions.modules_mut())
            .map_err(|e| self.errors.add(e));

        let module_id = if task.module.is_some() {
            Some(self.make_path_id(paths.module())?)
        } else {
            None
        };

        actions.add_run(TaskRunnerBuilder {
            print_id,
            realization_id,
            vars,
            copy_outputs_to,
            module_id,
            symlink_id: self.make_path_id(paths.link_src())?,
            link_target_id: self.make_path_id(paths.realization_relative())?,
            code: task.code,
        });

        Ok(true)
    }

    fn make_path_id(&mut self, path: &Path) -> Result<RunStrId> {
        let path_str = path.to_str().ok_or(PathEncodingError)?;
        self.wf.strings.run.intern(path_str)
    }
}

// INPUTS ///////////////////
impl TraversalResolver<'_> {
    /// true if any of this task's inputs are invalid, i.e. the task should run.
    fn handle_inputs(
        &mut self,
        task: &Node,
        inputs: &mut Vec<(IdentId, RunStrId)>,
        values: &RealInputs,
    ) -> Result<bool> {
        let mut should_run = false;
        for (k, v) in &task.vars.inputs {
            self.var_checker.insert(*k);
            let val = values.get(*v).ok_or(Error::MissingValue(*k, *v))?;

            match self.handle_input(val) {
                Ok((file_id, this_input_should_run)) => {
                    inputs.push((*k, file_id));
                    should_run = this_input_should_run || should_run;
                }
                Err(e) => self.var_err("input", *k, &task.key, e)?,
            }
        }
        Ok(should_run)
    }

    fn handle_input(&mut self, v: &RealInput) -> Result<(RunStrId, bool)> {
        match v {
            RealInput::Literal(lit_id) => {
                let lit_val = self.wf.strings.literals.get(*lit_id)?;
                let file_id = self.wf.strings.run.intern(lit_val)?;
                Ok((file_id, false))
            }
            RealInput::Task(task_id, output_id) => {
                let actual_id = self.deduper.get_actual_task_id(*task_id)?;
                let file_id = self.get_task_output_string(actual_id, *output_id)?;
                let antecedent_should_run = self.should_run[actual_id as usize];
                Ok((file_id, antecedent_should_run))
            }
        }
    }

    fn get_task_output_string(&self, t: ActualTaskId, o: IdentId) -> Result<RunStrId> {
        for (var_id, file_id) in &self.outputs[t as usize] {
            if *var_id == o {
                return Ok(*file_id);
            }
        }
        Err(Recapper::new(Error::TaskOutputNotFound(o)).into())
    }
}

// OUTPUTS /////////////////
impl TraversalResolver<'_> {
    fn handle_outputs(
        &mut self,
        task: &Node,
        outputs: &mut Vec<(IdentId, RunStrId)>,
        values: &RealOutputsParams,
        paths: &mut TaskDirPaths,
    ) -> Result<Vec<RunStrId>> {
        if task.module.is_some() {
            let mut copy_outputs_to = Vec::with_capacity(outputs.len());
            let mut outputs_metadata = Vec::with_capacity(outputs.len());
            for (k, v) in &task.vars.outputs {
                self.var_checker.insert(*k);
                let val = values.get(*v).ok_or(Error::MissingValue(*k, *v))?;

                match self.handle_module_output(val, paths) {
                    Ok((task_id, module_id)) => {
                        outputs.push((*k, module_id));
                        copy_outputs_to.push(task_id);
                        outputs_metadata.push((*k, task_id));
                    }
                    Err(e) => self.var_err("output", *k, &task.key, e)?,
                }
            }
            self.outputs.push(outputs_metadata);
            Ok(copy_outputs_to)
        } else {
            for (k, v) in &task.vars.outputs {
                self.var_checker.insert(*k);
                let val = values.get(*v).ok_or(Error::MissingValue(*k, *v))?;

                match self.handle_normal_output(val, paths) {
                    Ok(task_id) => outputs.push((*k, task_id)),
                    Err(e) => self.var_err("output", *k, &task.key, e)?,
                }
            }
            self.outputs.push(outputs.clone());
            Ok(Vec::with_capacity(0))
        }
    }

    fn handle_module_output(
        &mut self,
        val: &RealOutput,
        paths: &mut TaskDirPaths,
    ) -> Result<(RunStrId, RunStrId)> {
        let file = lit_str(val, self.wf, &self.wf.strings.literals, &mut self.strbuf)?;
        let task_id = path_id(paths.normal_output(file), &mut self.wf.strings.run)?;
        let module_id = path_id(paths.module_output(file), &mut self.wf.strings.run)?;
        Ok((task_id, module_id))
    }

    fn handle_normal_output(
        &mut self,
        val: &RealOutput,
        paths: &mut TaskDirPaths,
    ) -> Result<RunStrId> {
        let file = lit_str(val, self.wf, &self.wf.strings.literals, &mut self.strbuf)?;
        let task_id = path_id(paths.normal_output(file), &mut self.wf.strings.run)?;
        Ok(task_id)
    }
}

// PARAMS ///////////////////
impl TraversalResolver<'_> {
    fn handle_params(
        &mut self,
        task: &Node,
        params: &mut Vec<(IdentId, RunStrId)>,
        values: &RealOutputsParams,
    ) -> Result<()> {
        for (k, v) in &task.vars.params {
            self.var_checker.insert(*k);
            let val = values.get(*v).ok_or(Error::MissingValue(*k, *v))?;

            match lit_str(val, self.wf, &self.wf.strings.literals, &mut self.strbuf) {
                Ok(val_str) => {
                    let val_id = self.wf.strings.run.intern(val_str)?;
                    params.push((*k, val_id));
                }
                Err(e) => self.var_err("param", *k, &task.key, e)?,
            }
        }

        Ok(())
    }
}

impl TraversalResolver<'_> {
    /// store an error that was thrown while handling task variables:
    fn var_err(&mut self, ty: &str, k: IdentId, key: &RealTaskKey, e: anyhow::Error) -> Result<()> {
        let e = e.context(Recapper::new(ValueContext {
            ty: ty.to_owned(),
            ident: k,
            task: key.clone(),
        }));
        self.errors.add(e);
        Ok(())
    }
}

// we specify the lifetime for literals and strbuf,
// so we can be free to borrow other parts of self w/o trouble.
// NB this works for both outputs *and* params (not inputs tho).
fn lit_str<'a>(
    v: &RealOutput,
    wf: &Workflow,
    literals: &'a intern::TypedInterner<workflow::LiteralId, intern::LooseInterner<u8, u16>>,
    strbuf: &'a mut String,
) -> Result<&'a str> {
    match v {
        RealOutput::Literal(lit_id) => literals.get(*lit_id),
        RealOutput::Interp(lit_id, vars) => {
            strbuf.clear();
            wf.strings.make_interpolated(*lit_id, vars, strbuf)?;
            Ok(&*strbuf)
        }
    }
}

// more getting around borrow restrictions.
fn path_id(
    path: &Path,
    run_strs: &mut intern::TypedInterner<RunStrId, intern::PackedInterner>,
) -> Result<RunStrId> {
    let path_str = path.to_str().ok_or(PathEncodingError)?;
    run_strs.intern(path_str)
}
