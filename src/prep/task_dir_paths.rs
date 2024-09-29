use std::path::{Path, PathBuf};

use anyhow::Result;
use intern::GetStr;
use traverse::Node;
use workflow::{BranchStrs, Workflow};

use crate::fs::Fs;

/// Reusable container for common paths in the task realization directory.
pub struct TaskDirPaths {
    /// used for constructing other paths
    scratch: PathBuf,
    /// absolute path to the task realization dir
    realization: PathBuf,
    /// task realization dir, relative to the root task dir
    realization_relative: PathBuf,
    /// convenient symlink to the task realization dir (absolute)
    link_src: PathBuf,
    /// absolute path to module used by task, or empty if no module
    module: PathBuf,
}

impl TaskDirPaths {
    pub fn new() -> Self {
        Self {
            scratch: PathBuf::with_capacity(512),
            realization: PathBuf::with_capacity(512),
            link_src: PathBuf::with_capacity(512),
            realization_relative: PathBuf::with_capacity(512),
            module: PathBuf::with_capacity(512),
        }
    }

    pub fn make_paths(
        &mut self,
        task: &Node,
        wf: &Workflow,
        fs: &Fs,
        branch_strs: &mut BranchStrs,
        strbuf: &mut String,
    ) -> Result<()> {
        strbuf.clear();
        branch_strs.make_compact_string(&task.key.branch, wf, strbuf)?;
        fs.realization_relative(&*strbuf, &mut self.realization_relative);

        let base = fs.task_base(wf.strings.tasks.get(task.key.id)?, &mut self.scratch);

        fs.realization(base, &self.realization_relative, &mut self.realization);
        fs.link_src(
            base,
            branch_strs.get_or_insert(&task.key.branch, wf)?,
            &mut self.link_src,
        );

        self.module.clear();
        if let Some(module_id) = task.module {
            let path_str = wf.get_module_path(module_id)?;
            self.module.push(path_str);
        }

        Ok(())
    }

    pub fn realization(&self) -> &Path {
        &self.realization
    }

    pub fn realization_relative(&self) -> &Path {
        &self.realization_relative
    }

    pub fn link_src(&self) -> &Path {
        &self.link_src
    }

    pub fn module(&self) -> &Path {
        &self.module
    }

    pub fn normal_output(&mut self, file_relative: &str) -> &Path {
        self.scratch.clear();
        self.scratch.push(&self.realization);
        self.scratch.push(file_relative);
        &self.scratch
    }

    pub fn module_output(&mut self, file_relative: &str) -> &Path {
        self.scratch.clear();
        self.scratch.push(&self.module);
        self.scratch.push(file_relative);
        &self.scratch
    }

    /// return true if `exit_code` file exists and contains just the string "0".
    pub fn exit_code_success(&mut self, fs: &Fs, strbuf: &mut String) -> Result<bool> {
        let exit_code_file = fs.exit_code(&self.realization, &mut self.scratch);
        if fs.exists(exit_code_file) {
            fs.read_to_buf(exit_code_file, strbuf)?;
            if strbuf.trim() == "0" {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
