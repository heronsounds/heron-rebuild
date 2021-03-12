use anyhow::Result;

use util::{HashMap, Hasher};

use crate::Workflow;

use super::{string_fns, BranchSpec, Error};

/// Stores the "full" branch strings for branches we encounter while
/// preparing a workflow.
/// These strings are only used for logging and error messages.
#[derive(Debug)]
pub struct BranchStrs {
    strings: String,
    idxs: HashMap<BranchSpec, (u32, u32)>,
}

impl BranchStrs {
    /// Create a new `BranchStrs` with the given capacity and average length of a branch string.
    pub fn with_capacity_and_avg_len(cap: usize, avg_len: usize) -> Self {
        Self {
            idxs: HashMap::with_capacity_and_hasher(cap, Hasher::default()),
            strings: String::with_capacity(cap * avg_len),
        }
    }

    /// Get the stored branch string, or create one, store it and return it.
    pub fn get_or_insert(&mut self, branch: &BranchSpec, wf: &Workflow) -> &str {
        if let Some((start, end)) = self.idxs.get(branch) {
            &self.strings[*start as usize..*end as usize]
        } else {
            let start = self.strings.len();
            string_fns::make_full_string(branch, wf, &mut self.strings);
            let end = self.strings.len();
            self.idxs.insert(branch.clone(), (start as u32, end as u32));
            &self.strings[start..]
        }
    }

    /// Get the stored branch string, error if not found.
    pub fn get(&self, branch: &BranchSpec) -> Result<&str> {
        let (start, end) = self
            .idxs
            .get(branch)
            .ok_or_else(|| Error::NoBranchString(branch.clone()))?;
        Ok(&self.strings[*start as usize..*end as usize])
    }
}
