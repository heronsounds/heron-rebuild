use intern::GetStr;
use traverse::Node;
use util::{HashSet, Hasher};
use workflow::{IdentId, Workflow};

/// Checks that task variables are defined.
pub struct TaskVarChecker {
    vars: HashSet<IdentId>,
}

impl TaskVarChecker {
    /// Create a new `TaskVarChecker` with capacity (should be max vars expected from a single task).
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            vars: HashSet::with_capacity_and_hasher(cap, Hasher::default()),
        }
    }

    /// Clear out the internal hashset.
    pub fn clear(&mut self) {
        self.vars.clear();
    }

    /// Insert a variable to internal hashset.
    pub fn insert(&mut self, k: IdentId) {
        self.vars.insert(k);
    }

    /// Check that each variable used in execution code is defined.
    /// Currently, since checking for definitions could use some improvement,
    /// just prints a warning rather than erroring out.
    pub fn check(&self, node: &Node, wf: &Workflow) {
        for k in &node.code_vars {
            if !self.vars.contains(k) {
                let name = wf.strings.idents.get(*k);
                log::debug!(
                    "missing var {:?}: {name:?} (hope it's defined in the code...)",
                    *k
                );
            }
        }
    }
}
