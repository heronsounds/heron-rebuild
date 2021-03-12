use std::process::Command;

use intern::{GetStr, PackedInterner, TypedInterner};
use workflow::{IdentId, LiteralId, RunStrId, TaskVars, Workflow};

use super::TaskScriptBuilder;

/// Contains all information required to run a single task realization.
#[derive(Debug)]
pub struct TaskRunner {
    /// String that uniquely identifies this task (used for logging):
    pub print_id: RunStrId,
    /// The command to run
    pub cmd: Command,
    /// Id of the directory in which artifacts (`stdout.txt`, `task.sh`, `exit_code` etc.) will live.
    pub realization_dir: RunStrId,
    /// Ids of input files so we can verify they exist before execution.
    pub inputs: Vec<RunStrId>,
    /// Ids of output files so we can verify they exist after execution.
    pub outputs: Vec<RunStrId>,
    /// Ids of file paths to copy output files to (only used by module tasks).
    pub copy_outputs_to: Vec<RunStrId>,
}

/// Temporary struct for constructing a `TaskRunner`.
pub struct TaskRunnerBuilder {
    /// Id of directory in which artifacts live.
    pub realization_id: RunStrId,
    /// String that uniquely identifies this task, used for logging.
    pub print_id: RunStrId,
    /// Id of this task's module directory, if it has one.
    pub module_id: Option<RunStrId>,
    /// Id of the symlink that links to this task's realization directory
    /// (this is just for the user's convenience and isn't used anywhere else internally).
    pub symlink_id: RunStrId,
    /// Id of the link target (realization dir relative to the base task directory).
    pub link_target_id: RunStrId,
    /// list of variables that need to be defined for this task to run (inputs, outputs, & params).
    pub vars: TaskVars<(IdentId, RunStrId)>,
    /// If this task will execute in a module directory, ids of files to copy its outputs to.
    pub copy_outputs_to: Vec<RunStrId>,
    /// Id of string containing this task's execution code.
    pub code: LiteralId,
}

impl TaskRunnerBuilder {
    /// Convert self to a `TaskRunner` that can be run by the `WorkflowRunner`.
    pub fn into_task_runner(
        self,
        run_strs: &TypedInterner<RunStrId, PackedInterner>,
        wf: &Workflow,
        strbuf: &mut String,
    ) -> TaskRunner {
        // we will store inputs and outputs (so we can verify them before and after running),
        // but params can be discarded after we add them to the command.
        // however, we don't need the ident_ids, just the file handles.
        let mut inputs = Vec::with_capacity(self.vars.inputs.len());
        let mut outputs = Vec::with_capacity(self.vars.outputs.len());

        // cmd dir is either realization_dir, or module_dir if we're in a module:
        let cmd_dir: &str;
        // these are used for copying outputs from module back to realization dir;
        // if task doesn't use a module then we don't need them:
        let mut output_strs: Option<Vec<&str>>;

        if let Some(module_dir_id) = self.module_id {
            cmd_dir = run_strs.get(module_dir_id);
            output_strs = Some(Vec::with_capacity(self.vars.outputs.len()));
        } else {
            cmd_dir = run_strs.get(self.realization_id);
            output_strs = None;
        }

        // set up cmd and task.sh /////////////////////
        let mut cmd = Command::new("/usr/bin/env");
        cmd.arg("bash").arg("-xeuo").arg("pipefail");

        strbuf.clear();
        let mut script = TaskScriptBuilder::new(strbuf);

        cmd.current_dir(cmd_dir);
        script.write_prefix();

        // add inputs to cmd and task.sh /////////////
        for (id, file) in &self.vars.inputs {
            inputs.push(*file);
            let id = wf.strings.idents.get(*id);
            let file = run_strs.get(*file);
            cmd.env(id, file);
            script.write_assignment_line(id, file);
        }

        // add outputs to cmd and task.sh ///////////
        for (id, file) in &self.vars.outputs {
            outputs.push(*file);
            let id = wf.strings.idents.get(*id);
            let file = run_strs.get(*file);
            if let Some(vec) = output_strs.as_mut() {
                vec.push(file);
            }
            cmd.env(id, file);
            script.write_assignment_line(id, file);
        }

        // add params to cmd and task.sh ////////////
        for (id, file) in &self.vars.params {
            let id = wf.strings.idents.get(*id);
            let file = run_strs.get(*file);
            cmd.env(id, file);
            script.write_assignment_line(id, file);
        }

        // write actual code + suffix to cmd and task.sh ///
        let code = wf.strings.literals.get(self.code);
        if let Some(output_strs) = output_strs {
            let copy_strs: Vec<&str> = self
                .copy_outputs_to
                .iter()
                .map(|id| run_strs.get(*id))
                .collect();
            script.write_module_task_suffix(code, cmd_dir, &output_strs, &copy_strs);
        } else {
            script.write_normal_task_suffix(code);
        }
        cmd.arg("-c").arg(code);

        TaskRunner {
            cmd,
            print_id: self.print_id,
            realization_dir: self.realization_id,
            inputs,
            outputs,
            copy_outputs_to: self.copy_outputs_to,
        }
    }
}
