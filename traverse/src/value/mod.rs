mod real_value;
pub use real_value::{
    BranchMasks, PartialRealInput, RealInput, RealOutput, RealOutputOrParam, RealParam,
    RealValueLike,
};

mod value_resolver;
pub use value_resolver::ValueResolver;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Unsupported usage of literal value")]
    UnsupportedLiteral,
    #[error("Unsupported usage of task-output value")]
    UnsupportedTaskOutput,
    #[error("Unsupported usage of interpolated value")]
    UnsupportedInterp,
    #[error("Expected literal value, got {0}")]
    ExpectedLiteral(String),
    #[error("Specified branch does not exist")]
    BranchNotFound,
    #[error("Reference to nonexistent config value: {0}")]
    UndefinedConfigValue(String),
}
