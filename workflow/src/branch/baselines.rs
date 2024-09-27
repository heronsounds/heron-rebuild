use crate::{BranchpointId, IdentId};

/// Keeps track of which branch is baseline for each defined branch
/// in the workflow.
#[derive(Debug)]
pub struct BaselineBranches {
    vec: Vec<IdentId>,
}

impl BaselineBranches {
    /// Create a new `BaselineBranches` with the given capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            vec: Vec::with_capacity(cap),
        }
    }

    /// Set the given `IdentId` as the baseline for the given branchpoint.
    pub fn add(&mut self, k: BranchpointId, v: IdentId) {
        let k: usize = k.into();
        let len = self.vec.len();
        if k >= len {
            self.vec.reserve(k + 1);
            self.vec.append(&mut vec![crate::NULL_IDENT; k + 1 - len]);
        }
        let existing_v = &mut self.vec[k];
        if *existing_v == crate::NULL_IDENT {
            *existing_v = v;
        }
    }

    /// Get the `IdentId` of baseline branch for the given branchpoint.
    pub fn get(&self, k: BranchpointId) -> IdentId {
        let k: usize = k.into();
        self.vec[k]
    }

    /// Iterate through baseline branch values.
    // NB the first part of the tuple is equivalent to a BranchpointId.
    pub fn iter(&self) -> impl Iterator<Item = (usize, &IdentId)> {
        self.vec.iter().enumerate()
    }
}
