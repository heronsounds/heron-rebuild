/// Parse all the info in a traversal, dedupe, and prepare to start running.
mod traversal_resolver;
pub use traversal_resolver::TraversalResolver;

/// Clean up old runs and create directories used during execution.
mod pre_runner;
use pre_runner::Actions;
pub use pre_runner::PreRunner;

/// All the information needed to actually execute a task.
mod task_runner;
pub use task_runner::TaskRunner;
use task_runner::TaskRunnerBuilder;

/// Creates common paths in a task directory.
mod task_dir_paths;
use task_dir_paths::TaskDirPaths;

/// Utility for generating the `task.sh` file record.
mod task_script_builder;
use task_script_builder::TaskScriptBuilder;

/// Eliminate duplicate task realizations from a traversal.
mod deduper;
use deduper::Deduper;

/// Check that all variables in a task are defined.
mod task_var_checker;
use task_var_checker::TaskVarChecker;

/// Check that modules used by a traversal actually exist.
mod module_checker;
use module_checker::ModuleChecker;

/// index into vecs used by TraversalResolver:
type ActualTaskId = u16;

use traverse::{RealInput, RealOutputOrParam};

type RealInputs = util::IdVec<workflow::RealValueId, RealInput>;
type RealOutputsParams = util::IdVec<workflow::RealValueId, RealOutputOrParam>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Output value \"{0:?}\" not found")]
    TaskOutputNotFound(workflow::IdentId),
    #[error("Module dir does not exist: {0:?} (used by task \"{1:?}\"; path: {2})")]
    MissingModule(workflow::ModuleId, workflow::AbstractTaskId, String),
    #[error("Value does not exist: named {0:?}, id {1:?}")]
    MissingValue(workflow::IdentId, workflow::RealValueId),
    #[error("Attempted to get actual task id for nonexistent real task id: {0:?}")]
    MissingActualTaskId(workflow::RealTaskId),
}

impl workflow::Recap for Error {
    fn recap(&self, wf: &workflow::WorkflowStrings) -> anyhow::Result<Option<String>> {
        use intern::GetStr;
        match self {
            Self::MissingModule(m, t, path) => {
                let module = wf.modules.get(*m)?;
                let task = wf.tasks.get(*t)?;
                Ok(Some(format!(
                    "Module dir does not exist: {module} (used by task {task}; path: {path}"
                )))
            }
            Self::TaskOutputNotFound(o) => Ok(Some(format!(
                "Task output value not found: {}",
                wf.idents.get(*o)?
            ))),
            Self::MissingValue(ident, val_id) => Ok(Some(format!(
                "Value does not exist: named {}, id {val_id:?}",
                wf.idents.get(*ident)?,
            ))),
            _ => Ok(None),
        }
    }
}
