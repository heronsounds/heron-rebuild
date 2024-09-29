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
    idxs: HashMap<BranchSpec, (u16, u16)>,
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
    pub fn get_or_insert(&mut self, branch: &BranchSpec, wf: &Workflow) -> Result<&str> {
        if let Some((start, end)) = self.idxs.get(branch) {
            Ok(&self.strings[*start as usize..*end as usize])
        } else {
            let start = self.strings.len();
            string_fns::make_full_string(branch, wf, &mut self.strings)?;
            let end = self.strings.len();
            self.idxs.insert(branch.clone(), (start.try_into()?, end.try_into()?));
            Ok(&self.strings[start..])
        }
    }

    /// Get the stored branch string, error if not found.
    pub fn get(&self, branch: &BranchSpec) -> Result<&str> {
        let (start, end) =
            self.idxs.get(branch).ok_or_else(|| Error::NoBranchString(branch.clone()))?;
        Ok(&self.strings[*start as usize..*end as usize])
    }

    // Make a "compact" branch string; i.e. w/ "Baseline.baseline" filling in for actual
    // baseline branch specifications.
    // We put this method here for convenience and to avoid having to import the fn directly;
    // since the compact strings are only used once (when creating task realization paths),
    // we don't need to store them here.
    #[inline]
    pub fn make_compact_string(
        &self,
        branch: &BranchSpec,
        wf: &Workflow,
        buf: &mut String,
    ) -> Result<()> {
        string_fns::make_compact_string(branch, wf, buf)
    }
}
