mod abstract_value;
pub use abstract_value::Value;
use abstract_value::{BaseValue, DirectValue};

mod real_value;
pub use real_value::{
    BranchMasks, PartialRealInput, RealInput, RealOutput, RealOutputOrParam, RealParam,
    RealValueLike,
};

mod value_creation;
pub use value_creation::create_value;

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
    #[error("Reference to nonexistent config value: {0}")]
    NonexistentConfigValue(String),
}
