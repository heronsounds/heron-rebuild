use anyhow::Result;
use heron_rebuild::{App, Args};
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use tempfile::tempdir;

static MODULE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::default());

const MODULE_PATH: &str = "examples/test-module";

fn basic_args(output: String) -> Args {
    Args {
        config: String::from("examples/stub.tconf"),
        output: output,
        plan: None,
        tasks: Vec::with_capacity(0),
        invalidate: false,
        yes: true,
        verbose: 1,
        branch: Vec::with_capacity(0),
        baseline: false,
        dry_run: false,
    }
}

fn stringify_dir(dir: &tempfile::TempDir) -> String {
    dir.path().to_str().unwrap().to_owned()
}

fn run_basic() -> Result<tempfile::TempDir> {
    run_plan("debug")
}

fn run_plan(plan: &str) -> Result<tempfile::TempDir> {
    simple_logging::log_to_stderr(log::LevelFilter::Trace);
    // create module dir if it doesn't exist, but only once:
    {
        let _lock = MODULE_LOCK.lock();
        let module_dir = PathBuf::from(MODULE_PATH);
        if !module_dir.exists() {
            std::fs::create_dir(&module_dir)?;
        }
    }

    let output = tempdir()?;
    let mut args = basic_args(stringify_dir(&output));

    args.plan = Some(plan.to_owned());
    let settings = args.try_into()?;
    let app = App::new(settings);
    app.run()?;

    Ok(output)
}

#[test]
fn test_basic() -> Result<()> {
    let output = run_basic()?;

    let mut buf = PathBuf::from(output.path());
    buf.push("productbuild");
    buf.push("realizations");
    buf.push("Baseline.baseline");
    assert!(buf.exists(), "Goal task directory exists");
    buf.push("exit_code");
    assert!(buf.exists(), "Goal task exit_code file exists");

    output.close()?;
    Ok(())
}

#[test]
fn test_invalidate_goal_empty_branch() -> Result<()> {
    let output = run_basic()?;

    // invalidate goal node ("productbuild"), with no branch args:
    let mut args = basic_args(stringify_dir(&output));
    args.invalidate = true;
    args.tasks = vec![String::from("productbuild")];
    let settings = args.try_into()?;
    App::new(settings).run()?;

    let mut buf = PathBuf::from(output.path());
    buf.push("productbuild");
    buf.push("realizations");
    assert!(
        !buf.exists(),
        "Goal task realizations directory was deleted"
    );

    // re-run and confirm goal node now exists:
    let mut args = basic_args(stringify_dir(&output));
    args.plan = Some("debug".to_owned());
    let settings = args.try_into()?;
    App::new(settings).run()?;

    buf = PathBuf::from(output.path());
    buf.push("productbuild");
    buf.push("realizations");
    buf.push("Baseline.baseline");
    assert!(buf.exists(), "Goal task was recreated");

    output.close()?;
    Ok(())
}

#[test]
fn test_invalidate_goal_non_matching_branch() -> Result<()> {
    let output = run_basic()?;

    let mut args = basic_args(stringify_dir(&output));
    args.invalidate = true;
    args.tasks = vec![String::from("productbuild")];
    args.branch = vec!["Profile.release".to_owned()];
    let settings = args.try_into()?;
    App::new(settings).run()?;

    let mut buf = PathBuf::from(output.path());
    buf.push("productbuild/realizations/Baseline.baseline");
    assert!(buf.exists(), "Goal task was not deleted");

    output.close()?;
    Ok(())
}

#[test]
fn test_invalidate_earlier_task() -> Result<()> {
    let output = run_basic()?;
    let output_string = stringify_dir(&output);

    let mut args = basic_args(output_string.clone());
    args.invalidate = true;
    args.tasks = vec!["pkgbuild".to_owned()];
    args.branch = vec!["Framework.vst".to_owned()];
    let settings = args.try_into()?;
    App::new(settings).run()?;

    let mut dependent = PathBuf::from(output.path());
    dependent.push("productbuild/realizations/Baseline.baseline/exit_code");
    assert!(dependent.exists(), "Dependent task still exists");
    let dependent_created = dependent.metadata()?.created()?;

    let mut vst = PathBuf::from(output.path());
    vst.push("pkgbuild/realizations/Baseline.baseline+Framework.vst/exit_code");
    assert!(!vst.exists(), "Targeted task was deleted");

    let mut au = PathBuf::from(output.path());
    au.push("pkgbuild/realizations/Baseline.baseline/exit_code");
    assert!(
        au.exists(),
        "Other realizations of target task were not deleted"
    );
    let au_created = au.metadata()?.created()?;

    // now run it again:
    let mut args = basic_args(output_string);
    args.plan = Some("debug".to_owned());
    App::new(args.try_into()?).run()?;

    assert!(
        dependent.metadata()?.created()? > dependent_created,
        "Dependent task was recreated"
    );
    assert!(vst.exists(), "Target task was recreated");
    assert!(
        au.metadata()?.created()? == au_created,
        "Other realization was not recreated and still exists"
    );

    output.close()?;
    Ok(())
}

#[test]
fn test_invalidate_earlier_task_baseline() -> Result<()> {
    let output = run_basic()?;
    let output_string = stringify_dir(&output);

    let mut args = basic_args(output_string.clone());
    args.invalidate = true;
    args.tasks = vec!["pkgbuild".to_owned()];
    args.baseline = true;
    let settings = args.try_into()?;
    App::new(settings).run()?;

    let mut dependent = PathBuf::from(output.path());
    dependent.push("productbuild/realizations/Baseline.baseline/exit_code");
    assert!(dependent.exists(), "Dependent task still exists");
    let dependent_created = dependent.metadata()?.created()?;

    let mut vst = PathBuf::from(output.path());
    vst.push("pkgbuild/realizations/Baseline.baseline+Framework.vst/exit_code");
    assert!(vst.exists(), "Other realization of target was not deleted");
    let vst_created = vst.metadata()?.created()?;

    let mut au = PathBuf::from(output.path());
    au.push("pkgbuild/realizations/Baseline.baseline/exit_code");
    assert!(!au.exists(), "Target was deleted");

    // now run it again:
    let mut args = basic_args(output_string);
    args.plan = Some("debug".to_owned());
    App::new(args.try_into()?).run()?;

    assert!(
        dependent.metadata()?.created()? > dependent_created,
        "Dependent task was recreated"
    );
    assert!(au.exists(), "Target task was recreated");
    assert!(
        vst.metadata()?.created()? == vst_created,
        "Other realization was not recreated and still exists"
    );

    output.close()?;
    Ok(())
}

#[test]
fn test_plan_with_two_goals() -> Result<()> {
    // we had a bug where the "Arch" branchpoint wasn't being added
    // to tasks (like cargo_build) that require it when the task
    // was explicitly specified as a goal (i.e. not implicitly specified
    // as antecedent of a goal).
    // This leads to creating an extra "Profile.debug" realization w/o "Arch",
    // b/c the tasks weren't deduped properly.
    // Here, we're using the existence of the correct symlink as a proxy for
    // deduping the branched tasks correctly.
    // (it would be nice to write a more specific lower-level test for this).
    let output = run_plan("two_goals")?;

    let correct_symlink = output.path().join("cargo_build/Profile.debug+Arch.x64");
    let incorrect_symlink = output.path().join("cargo_build/Profile.debug");

    assert!(correct_symlink.exists(), "Fully specified symlink exists");
    assert!(
        !incorrect_symlink.exists(),
        "Incorrect, partially specified symlink does not exist"
    );

    Ok(())
}

#[test]
fn test_plan_with_two_subplans() -> Result<()> {
    let output = run_plan("two_subplans")?;

    let realization = output.path().join("cargo_build/Profile.debug+Arch.x64");
    assert!(realization.exists(), "implicit task realization exists");

    Ok(())
}
