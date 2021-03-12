use workflow::{BRANCH_DELIM, BRANCH_KV_DELIM};

use crate::args::Args;
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("no plan specified")]
    NoPlanSpecified,
    #[error("invalid branch flag '{0}' (should be formatted 'Key1.Val1[+Key2.Val2...]')")]
    InvalidBranchFlag(String),
    #[error("Invalid config path has no parent (should not happen)")]
    ConfigHasNoParent,
}

/// Action that should be taken by this command line invocation.
#[derive(Debug)]
pub enum Action {
    /// Default action: run the plan specified w/ -p.
    RunPlan(String),
    /// Invalidate pre-run target if -x|--invalidate specified.
    Invalidate,
}

/// Representation of '-b' and '-B' arg values
#[derive(Debug)]
pub enum ArgsBranch {
    /// When no branch is specified, we apply operations to all branches of a task.
    Empty,
    /// If baseline is explicitly specified, we only apply operations to the baseline.
    Baseline,
    /// If any branches are specified, we apply operations to all matching branches.
    Specified(Vec<(String, String)>),
}

/// Settings are like Args, except all the logic has
/// been applied so e.g. defaults are added in.
#[derive(Debug)]
pub struct Settings {
    pub config: PathBuf,
    pub output: PathBuf,
    pub yes: bool,
    pub verbose: bool,
    pub action: Action,
    pub branches: ArgsBranch,
    pub tasks: Vec<String>,
    pub dry_run: bool,
}

impl Settings {
    /// Get canonicalized parent dir of config file:
    pub fn config_parent_dir(&self) -> Result<&Path, Error> {
        let parent_dir = self.config.parent().ok_or(Error::ConfigHasNoParent)?;
        Ok(parent_dir)
    }
}

// only used for testing:
impl Default for Settings {
    fn default() -> Self {
        Self {
            config: PathBuf::from("x"),
            output: PathBuf::from("x"),
            yes: true,
            verbose: true,
            action: Action::RunPlan(String::from("x")),
            branches: ArgsBranch::Empty,
            tasks: Vec::with_capacity(0),
            dry_run: false,
        }
    }
}

impl TryFrom<Args> for Settings {
    type Error = anyhow::Error;
    fn try_from(args: Args) -> Result<Self, Self::Error> {
        let branches: ArgsBranch;
        if args.baseline || args.branch[..] == ["Baseline.baseline"] {
            branches = ArgsBranch::Baseline;
        } else if args.branch.is_empty() {
            branches = ArgsBranch::Empty;
        } else {
            let mut inner = Vec::with_capacity(8);
            for branch_arg in args.branch {
                for branch_kv in branch_arg.split(BRANCH_DELIM) {
                    let (k, v) = branch_kv
                        .split_once(BRANCH_KV_DELIM)
                        .ok_or_else(|| Error::InvalidBranchFlag(branch_arg.to_owned()))?;
                    inner.push((k.to_owned(), v.to_owned()));
                }
            }
            branches = ArgsBranch::Specified(inner);
        }

        let action = if args.invalidate {
            Action::Invalidate
        } else {
            let plan = args.plan.ok_or(Error::NoPlanSpecified)?;
            Action::RunPlan(plan)
        };

        Ok(Self {
            config: Path::new(&args.config).canonicalize()?,
            output: Path::new(&args.output).canonicalize()?,
            action,
            yes: args.yes,
            verbose: args.verbose,
            branches,
            tasks: args.tasks,
            dry_run: args.dry_run,
        })
    }
}
