use anyhow::Result;

use crate::{AbstractTaskId, BranchMask, BranchSpec, IdentId, LiteralId, RealTaskId};

use super::Error;

pub type RealOutput = RealOutputOrParam;
pub type RealParam = RealOutputOrParam;

/// Trait for a type that can be returned from the various `resolve_*` fns.
pub trait RealValueLike: Sized {
    /// Create a new literal value, if allowed.
    fn literal(_lit_id: LiteralId) -> Result<Self, Error> {
        Err(Error::UnsupportedLiteral)
    }

    /// Create a new interpolated string value, if allowed.
    fn interp(_lit_id: LiteralId, _vars: Vec<(IdentId, LiteralId)>) -> Result<Self, Error> {
        Err(Error::UnsupportedInterp)
    }

    /// Create a new task output value, if allowed.
    fn task(_task: AbstractTaskId, _ident: IdentId, _branch: BranchSpec) -> Result<Self, Error> {
        Err(Error::UnsupportedTaskOutput)
    }

    /// If value needs to return a branch
    /// (i.e. if it's a task output value that will be used to create a new node in a traversal),
    /// update that branch with new branch information generated during value resolution.
    fn update_branch(&mut self, _branch: &BranchSpec) {}

    /// If value is a literal, get its id.
    /// (used when this value is interpolated into some other value).
    fn get_literal_id(&self) -> Result<LiteralId, Error>;
}

/// A fully-realized input value
#[derive(Debug)]
pub enum RealInput {
    /// literally-specified input value
    Literal(LiteralId),
    /// input value taken from the output of another task
    Task(RealTaskId, IdentId),
}

/// A partially-realized input value,
/// which will soon be converted into a `RealInput`.
#[derive(Debug)]
pub enum PartialRealInput {
    /// literally-specified input value
    Literal(LiteralId),
    /// input value taken from the output of another task,
    /// that still needs to be converted to use a real task id.
    Task(AbstractTaskId, IdentId, BranchSpec),
}

impl RealValueLike for PartialRealInput {
    fn literal(lit_id: LiteralId) -> Result<Self, Error> {
        Ok(Self::Literal(lit_id))
    }

    fn task(task: AbstractTaskId, ident: IdentId, branch: BranchSpec) -> Result<Self, Error> {
        Ok(Self::Task(task, ident, branch))
    }

    fn update_branch(&mut self, branch: &BranchSpec) {
        if let Self::Task(_, _, existing_branch) = self {
            existing_branch.insert_all(branch);
        }
    }

    fn get_literal_id(&self) -> Result<LiteralId, Error> {
        match self {
            Self::Literal(id) => Ok(*id),
            _ => Err(Error::ExpectedLiteral(format!("{:?}", self))),
        }
    }
}

/// A fully-realized output or param value.
#[derive(Debug)]
pub enum RealOutputOrParam {
    /// literally-specified output or param value.
    Literal(LiteralId),
    /// Output or param value that interpolates other (literal) values.
    // idea: just a single struct, no variants, vec is empty if no interp vars? hm.
    Interp(LiteralId, Vec<(IdentId, LiteralId)>),
}

impl RealValueLike for RealOutputOrParam {
    fn literal(lit_id: LiteralId) -> Result<Self, Error> {
        Ok(Self::Literal(lit_id))
    }

    fn interp(lit_id: LiteralId, vars: Vec<(IdentId, LiteralId)>) -> Result<Self, Error> {
        Ok(Self::Interp(lit_id, vars))
    }

    fn get_literal_id(&self) -> Result<LiteralId, Error> {
        match self {
            Self::Literal(id) => Ok(*id),
            _ => Err(Error::ExpectedLiteral(format!("{:?}", self))),
        }
    }
}

/// for use in traversals while cleaning node branches:
#[derive(Debug, Default, Clone)]
pub struct BranchMasks {
    /// Branchpoints added at this node
    pub add: BranchMask,
    /// Branchpoints removed at this node (e.g. from a branch graft)
    pub rm: BranchMask,
}

impl BranchMasks {
    /// union this set of masks with another set.
    pub fn or_eq(&mut self, other: &Self) {
        self.add |= other.add;
        self.rm |= other.rm;
    }
}
