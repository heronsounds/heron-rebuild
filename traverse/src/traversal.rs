use anyhow::Result;

use intern::GetStr;
use util::{Bitmask, IdVec};
use workflow::{Plan, RealTaskKey, RealValueId, Workflow};

use super::{bfs, cleanup, Node};
use crate::value::{RealInput, RealOutputOrParam};

/// Represents a specific traversal through the tasks in the workflow.
pub struct Traversal {
    pub nodes: Vec<Node>,
    pub inputs: IdVec<RealValueId, RealInput>,
    pub outputs_params: IdVec<RealValueId, RealOutputOrParam>,
}

impl Traversal {
    /// The returned traversal may contain duplicates, but it is guaranteed
    /// to be ordered in run/dependency order and fully resolved with clean branches.
    pub fn create<B: Bitmask>(wf: &Workflow, plan: Plan) -> Result<Self> {
        debug_assert!(wf.strings.branchpoints.len() <= B::BITS);

        let mut traverser = bfs::BfsTraverser::<B>::new(wf);

        for plan in &plan.subplans {
            for goal in &plan.goals {
                for branch in &plan.branches {
                    let goal = RealTaskKey {
                        id: *goal,
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
            log::trace!("{}", wf.strings.get_real_task_str(&node.key)?,);
        }

        cleanup::clean_branches_reversed(&mut traversal, wf)?;

        traversal.errors.print_recap("building traversal", &wf.strings)?;
        Ok(cleanup::reverse_and_strip(traversal))
    }
}
