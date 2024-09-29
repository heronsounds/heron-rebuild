use anyhow::Result;

use intern::GetStr;
use util::{Bitmask, IdVec};
use workflow::{BranchStrs, Plan, RealInput, RealOutputOrParam, RealValueId, Workflow};

use super::{bfs, cleanup, Node, NodeBuilder, RealTaskKey};

/// Represents a specific traversal through the tasks in the workflow.
/// When this struct is returned from `create`, it may contain duplicates,
/// but it is guaranteed to be ordered in run/dependency order and fully resolved
/// with clean branches.
pub struct Traversal {
    pub nodes: Vec<Node>,
    pub inputs: IdVec<RealValueId, RealInput>,
    pub outputs_params: IdVec<RealValueId, RealOutputOrParam>,
    pub branch_strs: BranchStrs,
}

impl Traversal {
    pub fn create<B: Bitmask>(wf: &Workflow, plan: Plan) -> Result<Self> {
        debug_assert!(wf.strings.branchpoints.len() <= B::BITS);

        let mut traverser = bfs::BfsTraverser::<B>::new(wf);

        for plan in &plan.subplans {
            for goal in &plan.goals {
                for branch in &plan.branches {
                    let goal = RealTaskKey {
                        abstract_task_id: *goal,
                        branch: branch.clone(),
                    };
                    traverser.traverse(goal)?;
                }
            }
        }

        let mut traversal = traverser.into_traversal();

        log::debug!(
            "created unpruned traversal with {} nodes",
            traversal.nodes.len()
        );
        for node in &traversal.nodes {
            log::trace!(
                "{}[{}]",
                wf.strings.tasks.get(node.key.abstract_task_id),
                traversal.branch_strs.get(&node.key.branch)?,
            );
        }

        cleanup::clean_branches_reversed(&mut traversal, wf)?;

        let traversal = cleanup::reverse_and_strip(traversal);

        Ok(traversal)
    }
}

/// Builds a specific traversal through the tasks in the workflow.
pub struct TraversalBuilder<B> {
    /// ordered list of resolved nodes
    pub nodes: Vec<NodeBuilder<B>>,
    /// arena of input values
    pub inputs: IdVec<RealValueId, RealInput>,
    /// arena of output and param values (they have the same type constraints)
    pub outputs_params: IdVec<RealValueId, RealOutputOrParam>,
    /// store string representations of branches as we go:
    pub branch_strs: BranchStrs,
    /// indexes of root nodes:
    pub roots: Vec<u16>,
}
