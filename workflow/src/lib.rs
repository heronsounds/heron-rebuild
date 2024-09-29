mod strings;
use strings::WorkflowStrings;

mod value;
pub use value::{
    BranchMasks, PartialRealInput, RealInput, RealOutput, RealOutputOrParam, RealParam,
    RealValueLike, Value, ValueResolver,
};

mod task;
pub use task::{Task, TaskVars};

mod plan;
pub use plan::{Plan, Subplan};

mod branch;
pub use branch::{
    make_compact_string, parse_compact_branch_str, BaselineBranches, BranchSpec, BranchStrs,
};

mod id;
pub use id::{
    AbstractTaskId, AbstractValueId, BranchpointId, IdentId, LiteralId, ModuleId, RealTaskId,
    RealValueId, RunStrId, NULL_IDENT,
};

mod workflow;
pub use workflow::{SizeHints, Workflow};

pub type BranchMask = u8;

pub const BRANCH_KV_DELIM: char = '.';
pub const BRANCH_DELIM: char = '+';

// types and sizes:
// type IdSize = u8;

// type BRANCHPOINT_ADDR_SIZE = u8;
// type TASK_ADDR_SIZE = u16;
// type IDENT_ADDR_SIZE = u16;
// type MODULE_ADDR_SIZE = u8;
// type LITERAL_ADDR_SIZE = u16;
// type RUN_ADDR_SIZE = u16;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // TODO this needs more info about context:
    #[error("Unsupported feature: {0}")]
    Unsupported(String),
    #[error("Plan named \"{0}\" not found in config file")]
    PlanNotFound(String),
    #[error("Task defines multiple modules with '@'. Only one module is allowed.")]
    MultipleModulesDefined,
    #[error("Dot parameters (\".var\") are not yet supported")]
    DotParamsUnsupported,
    // #[error("Non-literal values not supported in interpolated variables")]
    // NonLiteralInterp,
    #[error("Matching branch not found (val: {0}, branch: {1}")]
    BranchNotFound(String, String),
    #[error("Unable to interpolate \"{0}\" into \"{1}\"")]
    Interp(String, String),
    #[error("Reference to nonexistent config value: {0}")]
    NonexistentConfigValue(String),
}
