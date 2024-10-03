use anyhow::Result;

use traverse::Node;
use util::{IdVec, PathEncodingError};
use workflow::{ModuleId, Recapper};

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
        module_ids_to_print: &mut Vec<ModuleId>,
    ) -> Result<()> {
        if let Some(module_id) = task.module {
            if !self.is_checked(module_id) {
                if fs.is_dir(paths.module())? {
                    self.checked.insert(module_id, true);
                    module_ids_to_print.push(module_id);
                    return Ok(());
                } else {
                    let module_path = paths.module().to_str().ok_or(PathEncodingError)?.to_owned();
                    return Err(Recapper::new(Error::MissingModule(
                        module_id,
                        task.key.id,
                        module_path,
                    ))
                    .into());
                }
            }
        }
        Ok(())
    }

    fn is_checked(&self, module_id: ModuleId) -> bool {
        self.checked.get(module_id).copied().unwrap_or(false)
    }
}
