use crate::{AbstractTaskId, BranchSpec, IdentId, LiteralId};

/// The base type of value, with no branching or grafting.
#[derive(Debug)]
pub enum BaseValue {
    /// A literal value
    Literal(LiteralId),
    /// A by-name reference to a config value defined elsewhere
    Config(IdentId),
    /// A reference to a task output using the task name and output var name
    Task(AbstractTaskId, IdentId),
    /// A literal string containing interpolated by-name references to config values defined elsewhere
    Interp(LiteralId, Vec<IdentId>),
}

/// A single (non-branching) right-hand-side value in a config file.
#[derive(Debug)]
pub enum DirectValue {
    /// A simple value that doesn't need to evaluate a branch.
    Simple(BaseValue),
    /// A value to be pulled from a specific branch.
    Graft(BaseValue, BranchSpec),
}

/// Any right-hand-side value in a workflow file.
#[derive(Debug)]
pub enum Value {
    /// Non-branching value
    Direct(DirectValue),
    /// Branching value with multiple `DirectValue`s defined for different branches
    Branched(Vec<(BranchSpec, DirectValue)>),
}
