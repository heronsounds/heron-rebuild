//!
//! The functions in this mod traverse the tasks in a `Workflow`, returning an ordered
//! list of tasks that can be run by structs in the `exec` mod.
//!
//! The traversal is created in 3 steps:
//! 1. Perform a BFS search backwards from the goal node(s), adding all necessary antecedent tasks.
//! 2. Reverse the list of tasks, correcting inter-task links.
//! 3. Step forward through the tasks, removing branchpoints that have been grafted out.
//!
//! In the end, you will have an ordered traversal of tasks
//! With only the minimal set of branchpoints required to uniquely identify each task.
//! The traversal may contain duplicate tasks, which will be removed in a later step.
//! Along the way, we partially resolve task variables (inputs, outputs, and params).
//! We only partially resolve them because we still don't know the
//! actual paths to task execution directories on disk; those paths will be provided by structs
//! in [`crate::prep`].

/// full BFS traversal (in reverse order) of the workflow
mod bfs;

/// reverse and simplify branches
mod cleanup;

/// struct returned by this mod
mod traversal;
pub use traversal::Traversal;

mod traversal_builder;
use traversal_builder::TraversalBuilder;

/// useful structs, including [`Node`]
mod node;
use node::NodeBuilder;
pub use node::{Node, RealTaskKey};

mod value;
pub use value::{RealInput, RealOutput, RealOutputOrParam, RealParam};

mod errors;
pub use errors::Errors;

// so we can have max ~16k task realizations, which should be enough.
// this is before deduping though, so realistically the cap is lower.
type NodeIdx = u16;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(
        "Out of node indices; {0} is greater than max index \
        (this may be due to a circular dependency)"
    )]
    OutOfIndices(usize),
    #[error("Task depends on itself: {0}")]
    ReflexiveTask(String),
}
