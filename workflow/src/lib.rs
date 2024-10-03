mod strings;
pub use strings::WorkflowStrings;

mod value;
pub use value::{BaseValue, DirectValue, Value};

mod task;
pub use task::{Task, TaskVars};

mod plan;
pub use plan::{Plan, Subplan};

mod branch;
pub use branch::{BaselineBranches, BranchSpec};

mod id;
pub use id::{
    AbstractTaskId, AbstractValueId, BranchpointId, IdentId, LiteralId, ModuleId, RealTaskId,
    RealValueId, RunStrId, NULL_IDENT,
};

mod error;
pub use error::{Errors, Recap, Recapper};

mod workflow;
pub use workflow::{SizeHints, Workflow};

mod string_cache;
pub use string_cache::{StringCache, StringMaker};

mod real_task;
pub use real_task::{RealTaskKey, RealTaskStrings};

// used to separate branchpoint from branch value e.g. "Profile.debug"
pub const BRANCH_KV_DELIM: char = '.';
// used to separate multiple branchpoint/value pairs e.g. "Profile.debug+Os.windows"
pub const BRANCH_DELIM: char = '+';

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Unsupported feature: {0}")]
    Unsupported(String),
    #[error("Plan not found: {0:?}")]
    PlanNotFound(IdentId),
    #[error("Task defines multiple modules with '@'. Only one module is allowed.")]
    MultipleModulesDefined,
    #[error("Dot parameters (\".var\") are not yet supported")]
    DotParamsUnsupported,
    #[error("Unable to interpolate \"{0}\" into \"{1}\"")]
    Interp(String, String),
    #[error("Plan is empty: '{0}'")]
    EmptyPlan(String),
    #[error("Module not found: {0:?}")]
    ModuleNotFound(ModuleId),
    #[error("Task not found: {0:?}")]
    TaskNotFound(AbstractTaskId),
    #[error("Value not found: {0:?}")]
    ValueNotFound(AbstractValueId),
}

impl Recap for Error {
    fn recap(&self, wf: &WorkflowStrings) -> anyhow::Result<Option<String>> {
        use intern::GetStr;
        match self {
            Self::ModuleNotFound(id) => {
                Ok(Some(format!("Module not found: {}", wf.modules.get(*id)?)))
            }
            Self::TaskNotFound(id) => Ok(Some(format!("Task not found: {}", wf.tasks.get(*id)?))),
            Self::PlanNotFound(id) => Ok(Some(format!(
                "Plan not found in config file: {}",
                wf.idents.get(*id)?
            ))),
            _ => Ok(None),
        }
    }
}
