use std::time::{SystemTime, SystemTimeError};

/// Utility for keeping track of the time it took to perform some operation.
pub struct Timer {
    start_time: SystemTime,
}

impl Timer {
    /// Create a new `Timer`.
    pub fn now() -> Self {
        Self {
            start_time: SystemTime::now(),
        }
    }

    /// Reset internal timer to now.
    pub fn reset(&mut self) {
        self.start_time = SystemTime::now();
    }

    /// Print a message with the elapsed time since the timer was last reset.
    pub fn print_elapsed(&self, task: &str) -> Result<(), SystemTimeError> {
        eprintln!("{} took {:?}", task, self.start_time.elapsed()?);
        Ok(())
    }
}
