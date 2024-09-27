use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;

use intern::{GetStr, InternStr};
use traverse::{Node, RealTaskKey, Traversal};
use util::PathEncodingError;
use workflow::{BranchStrs, IdentId, RealInput, RealOutput, RunStrId, TaskVars, Workflow};

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
        }
    }
}

impl TraversalResolver<'_> {
    /// NB should only be run once; if we want multiple runs for some reason need a reset fn.
    pub fn resolve_to_actions(&mut self, mut traversal: Traversal) -> Result<Actions> {
        let mut paths = TaskDirPaths::new();
        let mut actions = Actions::new(traversal.nodes.len());
        for task in &traversal.nodes {
            if self.deduper.is_dupe(&task.key) {
                continue;
            }

            let should_run = self
                .resolve_to_action(
                    task,
                    &traversal.inputs,
                    &traversal.outputs_params,
                    &mut actions,
                    &mut paths,
                    &mut traversal.branch_strs,
                )
                .with_context(|| {
                    let task_name = self.wf.strings.tasks.get(task.key.abstract_task_id);
                    format!("preparing task '{task_name}'")
                })?;

            self.should_run.push(should_run);
        }
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
        branch_strs: &mut BranchStrs,
    ) -> Result<bool> {
        self.var_checker.clear();
        paths.make_paths(task, self.wf, self.fs, branch_strs, &mut self.strbuf);
        let mut vars = TaskVars::new_with_sizes(&task.vars);

        // handle inputs and outputs first, since we need those even if task won't run:
        let invalidated = self.handle_inputs(task, &mut vars.inputs, inputs)?;
        let copy_outputs_to =
            self.handle_outputs(task, &mut vars.outputs, outputs_params, paths)?;

        let print_id = self.make_print_id(&task.key, branch_strs);
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
        self.var_checker.check(task, self.wf);
        self.module_checker
            .check(task, paths, self.fs, self.wf, actions.modules_mut())?;

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

    fn make_path_id(&mut self, path: &Path) -> Result<RunStrId, PathEncodingError> {
        let path_str = path.to_str().ok_or(PathEncodingError)?;
        Ok(self.wf.strings.run.intern(path_str))
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
            let val = values.get(*v);
            let (file_id, this_input_should_run) = self.handle_input(val).with_context(|| {
                format!("preparing task input '{}'", self.wf.strings.idents.get(*k))
            })?;
            inputs.push((*k, file_id));
            should_run = this_input_should_run || should_run;
        }
        Ok(should_run)
    }

    fn handle_input(&mut self, v: &RealInput) -> Result<(RunStrId, bool), Error> {
        match v {
            RealInput::Literal(lit_id) => {
                let lit_val = self.wf.strings.literals.get(*lit_id);
                let file_id = self.wf.strings.run.intern(lit_val);
                Ok((file_id, false))
            }
            RealInput::Task(task_id, output_id) => {
                let actual_id = self.deduper.get_actual_task_id(*task_id);
                let file_id = self.get_task_output_string(actual_id, *output_id)?;
                let antecedent_should_run = self.should_run[actual_id as usize];
                Ok((file_id, antecedent_should_run))
            }
        }
    }

    fn get_task_output_string(&self, t: ActualTaskId, o: IdentId) -> Result<RunStrId, Error> {
        for (var_id, file_id) in &self.outputs[t as usize] {
            if *var_id == o {
                return Ok(*file_id);
            }
        }
        let output_name = self.wf.strings.idents.get(o);
        Err(Error::TaskOutputNotFound(output_name.to_owned()))
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
                let val = values.get(*v);
                let (task_id, module_id) =
                    self.handle_module_output(val, paths).with_context(|| {
                        format!("preparing task output {}", self.wf.strings.idents.get(*k))
                    })?;

                outputs.push((*k, module_id));
                copy_outputs_to.push(task_id);
                outputs_metadata.push((*k, task_id));
            }
            self.outputs.push(outputs_metadata);
            Ok(copy_outputs_to)
        } else {
            for (k, v) in &task.vars.outputs {
                self.var_checker.insert(*k);
                let val = values.get(*v);
                let task_id = self.handle_normal_output(val, paths).with_context(|| {
                    format!("preparing task output {}", self.wf.strings.idents.get(*k))
                })?;

                outputs.push((*k, task_id));
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
            let val = values.get(*v);
            let val_str = lit_str(val, self.wf, &self.wf.strings.literals, &mut self.strbuf)
                .with_context(|| {
                    format!("preparing task param '{}'", self.wf.strings.idents.get(*k))
                })?;
            let val_id = self.wf.strings.run.intern(val_str);
            params.push((*k, val_id));
        }

        Ok(())
    }
}

impl TraversalResolver<'_> {
    /// make a user-friendly string for the task and intern it, returning its id.
    fn make_print_id(&mut self, key: &RealTaskKey, branch_strs: &mut BranchStrs) -> RunStrId {
        self.strbuf.clear();
        self.strbuf
            .push_str(&self.wf.strings.tasks.get(key.abstract_task_id).cyan());
        self.strbuf.push('[');
        self.strbuf
            .push_str(branch_strs.get_or_insert(&key.branch, self.wf));
        self.strbuf.push(']');
        self.wf.strings.run.intern(&self.strbuf)
    }
}

// we specify the lifetime for literals and strbuf,
// so we can be free to borrow other parts of self w/o trouble.
// NB this works for both outputs *and* params (not inputs tho).
fn lit_str<'a>(
    v: &RealOutput,
    wf: &Workflow,
    literals: &'a intern::TypedInterner<workflow::LiteralId, intern::LooseInterner<u8, u16>, u8>,
    strbuf: &'a mut String,
) -> Result<&'a str> {
    match v {
        RealOutput::Literal(lit_id) => Ok(literals.get(*lit_id)),
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
    Ok(run_strs.intern(path_str))
}
