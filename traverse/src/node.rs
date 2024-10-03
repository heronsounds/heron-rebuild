use workflow::{IdentId, LiteralId, ModuleId, RealTaskKey, RealValueId, Task, TaskVars};

use crate::value::BranchMasks;
use crate::NodeIdx;

/// Partially-resolved task used internally by traversal fns.
#[derive(Debug)]
pub struct NodeBuilder<B> {
    /// Unique id of the task contained in this node.
    pub key: RealTaskKey,
    /// traversal index of next task. if equal to this task's idx, the task is terminal.
    pub next_idx: NodeIdx,
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
    pub masks: BranchMasks<B>,
}

impl<B: Default> NodeBuilder<B> {
    /// Create a new Node with the given `key`, `next_idx`, and values
    /// copied from `task`.
    pub fn new(key: RealTaskKey, next_idx: NodeIdx, task: &Task) -> Self {
        NodeBuilder {
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

/// Resolved task exported to run subsystem
#[derive(Debug)]
pub struct Node {
    pub key: RealTaskKey,
    pub vars: TaskVars<(IdentId, RealValueId)>,
    pub code: LiteralId,
    pub code_vars: Vec<IdentId>,
    pub module: Option<ModuleId>,
}

impl<B> From<NodeBuilder<B>> for Node {
    fn from(node: NodeBuilder<B>) -> Self {
        Self {
            key: node.key,
            vars: node.vars,
            code: node.code,
            code_vars: node.code_vars,
            module: node.module,
        }
    }
}
