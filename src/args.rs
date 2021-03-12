use clap::Parser;

const CMD_NAME: &str = "hr";
const DEFAULT_CONFIG: &str = "rebuild.hr";
const DEFAULT_OUTPUT: &str = "output";

/// Stores our command-line args format.
#[derive(Parser)]
#[command(name = CMD_NAME, version, about = None, long_about = None)]
pub struct Args {
    /// Workflow definition file
    #[arg(short, long, value_name = "FILE", default_value = DEFAULT_CONFIG)]
    #[arg(env = "HERON_REBUILD_CONFIG")]
    pub config: String,

    /// Name of target plan
    #[arg(short, long, value_name = "PLAN")]
    pub plan: Option<String>,

    /// Name of target task
    #[arg(short, long = "task", value_name = "TASK")]
    pub tasks: Vec<String>,

    /// Invalidate specified task
    #[arg(short = 'x', long)]
    pub invalidate: bool,

    /// Output directory
    #[arg(short, long, value_name = "DIR", default_value = DEFAULT_OUTPUT)]
    #[arg(env = "HERON_REBUILD_OUTPUT")]
    pub output: String,

    /// Bypass user confirmation
    #[arg(short, long)]
    pub yes: bool,

    /// Print additional debugging info
    #[arg(short, long)]
    pub verbose: bool,

    /// Target branch
    #[arg(short, long, value_name = "K1.V1[+K2.V2]")]
    pub branch: Vec<String>,

    /// Use baseline branch ('-b Baseline.baseline')
    #[arg(short = 'B', long)]
    pub baseline: bool,

    /// Dry run; print info but don't modify anything.
    #[arg(short = 'n', long)]
    pub dry_run: bool,
}
