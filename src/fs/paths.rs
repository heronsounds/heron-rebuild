use std::path::{Path, PathBuf};

use super::Fs;

/// Utility fns for making common types of paths.
/// These fns are based on their callsite use pattern,
/// so sometimes a prefix will be included
/// and sometimes it's assumed that we'll add it here.
impl Fs {
    /// $OUTPUT/task_name
    pub fn task_base<'a>(&self, task: &str, buf: &'a mut PathBuf) -> &'a Path {
        self.parts2(&self.output_prefix, task, buf)
    }

    /// $OUTPUT/task_name/realizations
    pub fn realizations_dir<'a>(&self, task: &str, buf: &'a mut PathBuf) -> &'a Path {
        self.parts3(&self.output_prefix, task, "realizations", buf)
    }

    /// realizations/Branchpt.branch+Branchpt.branch
    pub fn realization_relative<'a>(&self, compact_branch: &str, buf: &'a mut PathBuf) -> &'a Path {
        self.parts2("realizations", compact_branch, buf)
    }

    /// $OUTPUT/task_name/realizations/Branchpt.branch
    pub fn realization(&self, base: &Path, link_target: &Path, buf: &mut PathBuf) {
        self.parts2(base, link_target, buf);
    }

    /// $OUTPUT/task_name/Branchpt.branch
    pub fn link_src(&self, base: &Path, full_branch: &str, buf: &mut PathBuf) {
        self.parts2(base, full_branch, buf);
    }

    /// $OUTPUT/branchpoints.txt
    pub fn branchpoints_txt<'a>(&self, buf: &'a mut PathBuf) -> &'a Path {
        self.parts2(&self.output_prefix, "branchpoints.txt", buf)
    }

    /// $OUTPUT/task_name/realizations/Branchpt.branch/exit_code
    pub fn exit_code<'a>(&self, realization: &Path, buf: &'a mut PathBuf) -> &'a Path {
        self.parts2(realization, "exit_code", buf)
    }

    /// $OUTPUT/task_name/realizations/Branchpt.branch/stdout.txt
    pub fn stdout<'a>(&self, realization: &str, buf: &'a mut PathBuf) -> &'a Path {
        self.parts2(realization, "stdout.txt", buf)
    }

    /// $OUTPUT/task_name/realizations/Branchpt.branch/stderr.txt
    pub fn stderr<'a>(&self, realization: &str, buf: &'a mut PathBuf) -> &'a Path {
        self.parts2(realization, "stderr.txt", buf)
    }

    /// $OUTPUT/task_name/realizations/Branchpt.branch/task.sh
    pub fn task_sh<'a>(&self, realization: &str, buf: &'a mut PathBuf) -> &'a Path {
        self.parts2(realization, "task.sh", buf)
    }

    /// $OUTPUT/task_name/realizations/Baseline.baseline
    pub fn baseline_realization<'a>(&self, task: &str, buf: &'a mut PathBuf) -> &'a Path {
        buf.clear();
        buf.push(&self.output_prefix);
        buf.push(task);
        buf.push("realizations");
        buf.push("Baseline.baseline");
        &*buf
    }

    fn parts2<'a, T, U>(&self, p1: T, p2: U, buf: &'a mut PathBuf) -> &'a Path
    where
        T: AsRef<Path>,
        U: AsRef<Path>,
    {
        buf.clear();
        buf.push(p1);
        buf.push(p2);
        &*buf
    }

    fn parts3<'a, T, U, V>(&self, p1: T, p2: U, p3: V, buf: &'a mut PathBuf) -> &'a Path
    where
        T: AsRef<Path>,
        U: AsRef<Path>,
        V: AsRef<Path>,
    {
        buf.clear();
        buf.push(p1);
        buf.push(p2);
        buf.push(p3);
        &*buf
    }
}
