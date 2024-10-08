mod baselines;
pub use baselines::BaselineBranches;

mod spec;
pub use spec::BranchSpec;

mod string_fns;
pub use string_fns::{parse_compact_branch_str, CompactBranchStrings, FullBranchStrings};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("No cached string for branch {0:?}")]
    NoBranchString(BranchSpec),
    #[error("invalid branch string: {0}")]
    InvalidBranchString(String),
    #[error("Invalid branchpoints.txt file")]
    InvalidBranchpointsFile,
    #[error("Branch is too large to fit in bitmap of size {0}: {1:?}")]
    BranchOutOfBounds(usize, BranchSpec),
}
