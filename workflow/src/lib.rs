mod strings;
use strings::WorkflowStrings;

mod value;
pub use value::{BaseValue, DirectValue, Value};

mod task;
pub use task::{Task, TaskVars};

mod plan;
pub use plan::{Plan, Subplan};

mod branch;
pub use branch::{BaselineBranches, BranchSpec, BranchStrs};

mod id;
pub use id::{
    AbstractTaskId, AbstractValueId, BranchpointId, IdentId, LiteralId, ModuleId, RealTaskId,
    RealValueId, RunStrId, NULL_IDENT,
};

mod workflow;
pub use workflow::{SizeHints, Workflow};

// used to separate branchpoint from branch value e.g. "Profile.debug"
pub const BRANCH_KV_DELIM: char = '.';
// used to separate multiple branchpoint/value pairs e.g. "Profile.debug+Os.windows"
pub const BRANCH_DELIM: char = '+';

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
    #[error("Unable to interpolate \"{0}\" into \"{1}\"")]
    Interp(String, String),
    #[error("{0} does not exist: '{1}'")]
    ItemNotFound(String, String),
    #[error("Plan is empty: '{0}'")]
    EmptyPlan(String),
}
