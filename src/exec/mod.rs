/// Runs the workflow
mod workflow_runner;
pub use workflow_runner::WorkflowRunner;

/// Run a subprocess
mod run_cmd;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Expected file not found: {0}")]
    ExpectedFileNotFound(String),
    #[error("Subprocess failed")]
    SubprocessFailed,
}
