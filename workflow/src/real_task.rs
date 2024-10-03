use anyhow::Result;
use colored::Colorize;

use crate::{AbstractTaskId, BranchSpec, StringMaker, WorkflowStrings};

/// Unique id of a resolved (real) task: an abstract task id
/// plus the branch that resolves it to an actual task.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RealTaskKey {
    pub id: AbstractTaskId,
    pub branch: BranchSpec,
}

#[derive(Debug)]
pub struct RealTaskStrings;

impl StringMaker<RealTaskKey> for RealTaskStrings {
    fn make_string(
        &self,
        task: &RealTaskKey,
        wf: &WorkflowStrings,
        buf: &mut String,
    ) -> Result<()> {
        use intern::GetStr;
        buf.push_str(&format!("{}", wf.tasks.get(task.id)?.cyan()));
        buf.push('[');
        buf.push_str(&wf.get_full_branch_str(&task.branch)?);
        buf.push(']');
        Ok(())
    }
}
