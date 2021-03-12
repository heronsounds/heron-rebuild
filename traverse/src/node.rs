use workflow::{
    AbstractTaskId, BranchMasks, BranchSpec, IdentId, LiteralId, ModuleId, RealValueId, Task,
    TaskVars,
};

/// Unique id of a resolved (real) task: an abstract task id
/// plus the branch that resolves it to an actual task.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RealTaskKey {
    pub abstract_task_id: AbstractTaskId,
    pub branch: BranchSpec,
}

/// Partially-resolved task used internally by traversal fns.
#[derive(Debug)]
pub struct Node {
    /// Unique id of the task contained in this node.
    pub key: RealTaskKey,
    /// traversal index of next task. if equal to this task's idx, the task is terminal.
    pub next_idx: usize, // u32?
    /// true if this task has no antecedents.
    pub is_root: bool,
    /// inputs, outputs, and params for this task.
    pub vars: TaskVars<(IdentId, RealValueId)>,
    /// code to run.
    pub code: LiteralId,
    /// vars referenced in code (so we can check them later).
    pub code_vars: Vec<IdentId>,
    /// optional module to run this task in.
    pub module: Option<ModuleId>,
    /// branches added and removed at this task.
    pub masks: BranchMasks,
}

impl Node {
    /// Create a new Node with the given `key`, `next_idx`, and values
    /// copied from `task`.
    pub fn new(key: RealTaskKey, next_idx: usize, task: &Task) -> Self {
        Node {
            key,
            next_idx,
            code: task.code,
            code_vars: task.referenced_vars.clone(),
            module: task.module,
            vars: TaskVars::new_with_sizes(&task.vars),
            masks: BranchMasks::default(),
            // NB we will set this to false if we find an antecedent during handling:
            is_root: true,
        }
    }
}
