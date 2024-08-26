use anyhow::Result;

use util::IdVec;
use workflow::{BranchStrs, RealInput, RealOutputOrParam, RealValueId, Workflow, Plan};

use super::{bfs, cleanup, Node, RealTaskKey};

// unlikely we'll ever have more than 255 roots, but we can increase this in future if needed:
type RootCount = u8;

/// Represents a specific traversal through the tasks in the workflow.
/// At this time, we only allow a single, simple plan as the goal for a traversal,
/// with a single task, but we will allow more complex plans in the future.
pub struct Traversal {
    /// ordered list of resolved nodes
    pub nodes: Vec<Node>,
    /// arena of input values
    pub inputs: IdVec<RealValueId, RealInput>,
    /// arena of output and param values (they have the same type constraints)
    pub outputs_params: IdVec<RealValueId, RealOutputOrParam>,
    /// number of root nodes (so we know when to stop looking for them)
    pub num_roots: RootCount,
    /// store string representations of branches as we go:
    pub branch_strs: BranchStrs,
}

impl Traversal {
    /// Create a new workflow traversal terminating with the tasks defined in the given `plan`.
    pub fn create(wf: &Workflow, plan: Plan, verbose: bool) -> Result<Self> {
        // let plan = wf.get_plan(plan)?;

        let mut traverser = bfs::BfsTraverser::new(wf, verbose);

        for goal in &plan.goals {
            for branch in &plan.branches {
                let goal = RealTaskKey {
                    abstract_task_id: *goal,
                    branch: branch.clone(),
                };
                traverser.traverse(goal)?;
            }
        }

        // clean up:
        // TODO there's a bug w/ multiple goal nodes right now,
        // where we end up with an extra task w/ some branches unspecified...
        // recreate it by adding cargo_build to au plan in find_fix...
        let mut traversal = traverser.into_traversal();
        cleanup::reverse(&mut traversal);
        cleanup::clean_branches(&mut traversal, wf, verbose)?;

        Ok(traversal)
    }
}
