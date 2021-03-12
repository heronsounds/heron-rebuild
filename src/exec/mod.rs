/// Runs the workflow
mod workflow_runner;
pub use workflow_runner::WorkflowRunner;

/// Run a subprocess
mod run_cmd;

// /// Encapsulates a single task execution
// mod task_runner;
// pub use task_runner::TaskRunner;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Expected file not found: {0}")]
    ExpectedFileNotFound(String),
    #[error("Subprocess failed")]
    SubprocessFailed,
}
