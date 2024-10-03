use util::IdVec;
use workflow::{Errors, RealValueId};

use crate::value::{RealInput, RealOutputOrParam};
use crate::{NodeBuilder, NodeIdx};

/// Builds a specific traversal through the tasks in the workflow.
pub struct TraversalBuilder<B> {
    /// ordered list of resolved nodes
    pub nodes: Vec<NodeBuilder<B>>,
    /// arena of input values
    pub inputs: IdVec<RealValueId, RealInput>,
    /// arena of output and param values (they have the same type constraints)
    pub outputs_params: IdVec<RealValueId, RealOutputOrParam>,
    /// indexes of root nodes:
    pub roots: Vec<NodeIdx>,
    /// for storing errors encountered during traversal:
    pub errors: Errors,
}
