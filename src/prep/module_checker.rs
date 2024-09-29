use anyhow::Result;

use intern::GetStr;
use traverse::Node;
use util::{IdVec, PathEncodingError};
use workflow::{ModuleId, Workflow};

use crate::fs::Fs;

use super::{Error, TaskDirPaths};

/// Checks that modules exist.
pub struct ModuleChecker {
    checked: IdVec<ModuleId, bool>,
}

impl ModuleChecker {
    /// Create a new `ModuleChecker` with the given capacity;
    /// should be equal to the number of modules in a workflow.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            checked: IdVec::fill(false, cap),
        }
    }

    /// Ok if module exists, or no module used.
    /// Adds module id to `module_ids_to_print`, only if this is the first time we've seen it.
    pub fn check(
        &mut self,
        task: &Node,
        paths: &TaskDirPaths,
        fs: &Fs,
        wf: &Workflow,
        module_ids_to_print: &mut Vec<ModuleId>,
    ) -> Result<()> {
        if let Some(module_id) = task.module {
            if !*self.checked.get(module_id).unwrap() {
                if fs.is_dir(paths.module())? {
                    self.checked.insert(module_id, true);
                    module_ids_to_print.push(module_id);
                    return Ok(());
                } else {
                    let module_name = wf.strings.modules.get(module_id)?.to_owned();
                    let task_name = wf.strings.tasks.get(task.key.id)?.to_owned();
                    let module_path = paths.module().to_str().ok_or(PathEncodingError)?.to_owned();
                    return Err(Error::MissingModule(module_name, task_name, module_path).into());
                }
            }
        }
        Ok(())
    }
}
