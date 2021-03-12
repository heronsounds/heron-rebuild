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

    // SET UP LOGGING /////////////////
    let log_level = if settings.verbose {
        log::LevelFilter::Info
    } else {
        std::env::set_var("RUST_BACKTRACE", "1");
        log::LevelFilter::Warn
    };
    simple_logging::log_to_stderr(log_level);

    // RUN THE THING /////////////////
    let app = App::new(settings);
    app.run()?;

    Ok(())
}
