use colored::Colorize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0} failed due to {1} errors")]
    AggregatedErrors(String, usize),
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
    pub fn print_recap(&self, label: &str) -> Result<(), Error> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            eprintln!("\nEncountered errors while {label}:\n");
            for e in &self.errors {
                eprintln!("{}: {e:?}\n", "ERROR".red());
            }
            Err(Error::AggregatedErrors(label.to_owned(), self.errors.len()))
        }
    }
}
