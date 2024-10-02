mod real_value;
pub use real_value::{
    BranchMasks, PartialRealInput, RealInput, RealOutput, RealOutputOrParam, RealParam,
    RealValueLike,
};

mod value_resolver;
pub use value_resolver::ValueResolver;

use workflow::{IdentId, Recap};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Literal value not supported in this position")]
    UnsupportedLiteral,
    #[error("Task-output value not supported in this position")]
    UnsupportedTaskOutput,
    #[error("Variable interpolation not supported in this position")]
    UnsupportedInterp,
    #[error("Expected literal value, got {0}")]
    ExpectedLiteral(String),
    #[error("Specified branch does not exist")]
    BranchNotFound,
    #[error("Reference to nonexistent config value: {0:?}")]
    UndefinedConfigValue(IdentId),
}

impl Recap for Error {
    fn recap(&self, wf: &workflow::WorkflowStrings) -> anyhow::Result<Option<String>> {
        use intern::GetStr;
        match self {
            Self::UndefinedConfigValue(id) => Ok(Some(format!(
                "Reference to nonexistent config value: {}",
                wf.idents.get(*id)?,
            ))),
            _ => Ok(None),
        }
    }
}

// #[derive(Debug)]
// pub enum ValueType {
//     Input, Output, Param,
// }

/// for more helpful error messages
#[derive(Debug)]
pub struct ValueContext {
    pub ty: String,
    pub task: workflow::AbstractTaskId,
    pub ident: workflow::IdentId,
}

impl Recap for ValueContext {
    fn recap(&self, wf: &workflow::WorkflowStrings) -> anyhow::Result<Option<String>> {
        use colored::Colorize;
        use intern::GetStr;
        Ok(Some(format!(
            "Invalid {} '{}' in task '{}'",
            self.ty,
            wf.idents.get(self.ident)?.yellow(),
            wf.tasks.get(self.task)?.cyan(),
            // TODO we lost the branch string when moving to recapper,
            // that needs to be in WorkflowStrings.
            // probably wrapped in an Rc?
            // while we're at it, just store a mapping from RealTaskKey too?
        )))
    }
}
