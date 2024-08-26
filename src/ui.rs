use std::cell::RefCell;

use anyhow::Result;
use colored::Colorize;

use util::Timer;

use crate::settings::Settings;

/// All interactions with the text UI should go through this struct.
pub struct Ui {
    /// -v setting, displays extra text info to user
    pub verbose: bool,
    /// -y setting, ignores all points where the user is prompted to enter 'y'
    override_confirmation: bool,
    /// keeps track of time for each task
    timer: Timer,
    /// buffer to hold strings internally when getting input
    strbuf: RefCell<String>,
}

impl Ui {
    pub fn new(settings: &Settings) -> Self {
        Self {
            verbose: settings.verbose,
            override_confirmation: settings.yes,
            timer: Timer::now(),
            // Refcell so we can call confirm() w/o needing a unique reference:
            strbuf: RefCell::new(String::with_capacity(16)),
        }
    }

    pub fn confirm(&self, prompt: &str) -> Result<bool> {
        if self.override_confirmation {
            return Ok(true);
        }
        eprintln!("{} (y/N)", prompt);

        let mut strbuf = self.strbuf.borrow_mut();

        strbuf.clear();
        std::io::stdin().read_line(&mut strbuf)?;
        match strbuf.chars().next() {
            Some('y') => Ok(true),
            _ => Ok(false),
        }
    }

    pub fn start_timer(&mut self) {
        if self.verbose {
            self.timer.reset();
        }
    }

    pub fn print_elapsed(&mut self, task: &str) -> Result<(), std::time::SystemTimeError> {
        if self.verbose {
            self.timer.print_elapsed(task)
        } else {
            Ok(())
        }
    }

    pub fn verbose_msg(&self, msg: &str) {
        if self.verbose {
            eprintln!("{}", msg);
        }
    }

    pub fn verbose_progress(&self, msg: &str) {
        if self.verbose {
            eprint!("{}... ", msg.magenta());
        }
    }

    // pub fn verbose_progress_display<T: std::fmt::Display>(&self, msg: &str, arg: T) {
    //     if self.verbose {
    //         eprint!("{} {}...", msg.magenta(), arg);
    //     }
    // }

    pub fn verbose_progress_debug<T: std::fmt::Debug>(&self, msg: &str, arg: T) {
        if self.verbose {
            eprint!("{} {:?}... ", msg.magenta(), arg);
        }
    }

    pub fn done(&self) {
        if self.verbose {
            eprintln!("{}.", "done".green());
        }
    }
}
