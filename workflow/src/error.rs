use anyhow::Result;
use colored::Colorize;

use crate::WorkflowStrings;

/// For re-throwing after we've printed a list of errors to the user.
#[derive(Debug, thiserror::Error)]
#[error("{0} failed due to {1} errors")]
pub struct AggregatedErrors(pub String, pub usize);

/// impl this for error traits that rely on Workflow strings for their message.
pub trait Recap: std::fmt::Debug + Send + Sync {
    fn recap(&self, wf: &WorkflowStrings) -> Result<Option<String>>;
}

/// Wrap errors in this struct at the call site so they can use the WorkflowStrings
/// object in the recap.
#[derive(Debug, thiserror::Error)]
#[error("{e:?}")]
pub struct Recapper {
    e: Box<dyn Recap>,
}

impl Recapper {
    pub fn new(e: impl Recap + 'static) -> Self {
        Self { e: Box::new(e) }
    }
}

// in future we can add a `warnings` field, too.
pub struct Errors {
    errors: Vec<anyhow::Error>,
}

impl Default for Errors {
    fn default() -> Self {
        Self {
            // ideally we won't have any,
            // and we don't mind reallocating if we're already in an error state:
            errors: Vec::with_capacity(0),
        }
    }
}

impl Errors {
    pub fn add_context(&mut self, e: anyhow::Error, msg: String) {
        log::trace!("{msg}: {e:?}");
        self.errors.push(e.context(msg));
    }

    pub fn add(&mut self, e: anyhow::Error) {
        log::trace!("error: {e:?}");
        self.errors.push(e);
    }

    /// Print full list of errors to stderr, fail w/ an aggregated error
    /// if there were one or more errors.
    pub fn print_recap(&self, label: &str, wf: &WorkflowStrings) -> Result<()> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            eprintln!("\n{} {}:\n", "Encountered errors while".red(), label.red());
            for e in &self.errors {
                use anyhow::Context;
                recap(e, wf).context("Unable to print error list due to errors while printing")?;
            }
            Err(AggregatedErrors(label.to_owned(), self.errors.len()).into())
        }
    }
}

fn recap(e: &anyhow::Error, wf: &WorkflowStrings) -> Result<()> {
    eprint!("{}: ", "ERROR".red());

    handle_recapper_anyhow(e, wf)?;
    for cause in e.chain().skip(1) {
        eprint!("\nCaused by:\n\t");
        handle_recapper_dyn(cause, wf)?;
    }
    eprintln!();
    Ok(())
}

// both anyhow::Error and std Error have a fn called `downcast_ref`, but they aren't the
// same method, so we need two fns to handle them.
fn handle_recapper_dyn(e: &(dyn std::error::Error + 'static), wf: &WorkflowStrings) -> Result<()> {
    if let Some(recapper) = e.downcast_ref::<Recapper>() {
        if let Some(msg) = recapper.e.recap(wf)? {
            eprintln!("{}", msg);
            return Ok(());
        }
    }
    eprintln!("{}", e);
    Ok(())
}

fn handle_recapper_anyhow(e: &anyhow::Error, wf: &WorkflowStrings) -> Result<()> {
    if let Some(recapper) = e.downcast_ref::<Recapper>() {
        if let Some(msg) = recapper.e.recap(wf)? {
            eprintln!("{}", msg);
            return Ok(());
        }
    }
    eprintln!("{}", e);
    Ok(())
}
