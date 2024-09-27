/// High-level command line app
mod app;
/// Definition of command-line args
mod args;
/// Workflow execution
mod exec;
/// Filesystem operations
mod fs;
/// Structs for preparing a workflow to run
mod prep;
/// Combined command-line and config file run settings
mod settings;
/// Text UI
mod ui;

mod invalidate;

// exported for tests:
pub use app::App;
pub use args::Args;
pub use settings::Settings;

/// Run the command-line app.
pub fn run() -> Result<(), anyhow::Error> {
    use clap::Parser;
    let args = Args::parse();

    // INTERPRET SETTINGS ///////////////
    let settings: Settings = args.try_into()?;

    let log_level = match settings.verbose {
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    // TODO why did I add this here?
    std::env::set_var("RUST_BACKTRACE", "1");
    simple_logging::log_to_stderr(log_level);

    // RUN THE THING /////////////////
    let app = App::new(settings);
    app.run()?;

    Ok(())
}

// TODO from building find-fix:
// - runtime error messages shd say which task they're from
// - "Task completed" shd say which task, too.
// - upgrade plans!
// - cryptic error if module dir doesn't exist, and happens too early.
// - something like '-r' to force re-run a task. (or '-f'...)
// - and maybe '-R' to force re-run task and *all its dependencies*...
// - need an equivalent to ducttape's 'mark_done' too.
// - when force running a task, need '-A|--all' to specify all branches? maybe change -x behavior too?
