use util::{Bitmask, IdVec};

use crate::{BranchpointId, IdentId, NULL_IDENT};

use super::Error;

/// Represents a branch: a list of (branchpoint, branch value) pairs.
/// If a branch has the `NULL_IDENT` `IdentId`, that means it is a
/// baseline branch.
/// Baseline can mean either that the branch was unspecified, or that it was
/// specifically intended to be baseline, depending on the use case.
/// This ambiguity is something we should clean up eventually.
#[derive(Debug, Default, Clone, Hash, PartialEq, Eq)]
pub struct BranchSpec {
    branches: IdVec<BranchpointId, IdentId>,
}

impl BranchSpec {
    /// Create a new branch with the given branchpoint/branch pair.
    pub fn simple(k: BranchpointId, v: IdentId) -> Self {
        let mut branches = IdVec::with_capacity(usize::from(k) + 1);
        branches.insert(k, v);
        Self { branches }
    }

    /// Insert the given branchpoint/branch pair into this BranchSpec.
    #[inline]
    pub fn insert(&mut self, k: BranchpointId, v: IdentId) {
        self.branches.insert(k, v);
    }

    /// Get the branch id if it is specified/non-baseline, otherwise None.
    pub fn get_specified(&self, k: BranchpointId) -> Option<IdentId> {
        if usize::from(k) < self.branches.len() {
            none_if_baseline(*self.branches.get(k).expect("Requested branchpoint does not exist"))
        } else {
            None
        }
    }

    /// true if branchpoint k is unspecified/baseline.
    pub fn is_unspecified(&self, k: BranchpointId) -> bool {
        usize::from(k) >= self.branches.len()
            || *self.branches.get(k).expect("Requested branchpoint does not exist") == NULL_IDENT
    }

    /// true if branchpoint k is specified/non-baseline.
    pub fn is_specified(&self, k: BranchpointId) -> bool {
        usize::from(k) < self.branches.len()
            && *self.branches.get(k).expect("Requested branchpoint does not exist") != NULL_IDENT
    }

    /// remove branch info for branchpoint k, leaving it unspecified/baseline.
    pub fn unset(&mut self, k: BranchpointId) {
        if usize::from(k) < self.branches.len() {
            self.branches.insert(k, NULL_IDENT)
        }
    }

    /// Iterate through branchpoints
    pub fn iter(&self) -> std::slice::Iter<'_, IdentId> {
        self.branches.iter()
    }

    /// Length of the underlying vec.
    pub fn len(&self) -> usize {
        self.branches.len()
    }

    /// True if len == 0.
    pub fn is_empty(&self) -> bool {
        self.branches.is_empty()
    }

    /// true if all specified branches in this branch match with other branch;
    /// allowing any unspecified branches here to still count as a match.
    pub fn is_compatible(&self, other: &Self) -> bool {
        for (k, v) in self.iter_specified() {
            if let Some(v) = v {
                if let Some(other_v) = other.get_specified(k.into()) {
                    if other_v != v {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// true if all specified branches in this branch exactly match other branch.
    pub fn is_exact_match(&self, other: &Self) -> bool {
        for (k, v) in self.iter_specified() {
            if let Some(v) = v {
                let other_v = other.get_specified(k.into());
                if other_v.is_none() || other_v.unwrap() != v {
                    return false;
                }
            }
        }
        true
    }

    /// Insert all defined branches from `other` into `self`.
    pub fn insert_all(&mut self, other: &Self) {
        for (k, v) in other.branches.iter().enumerate() {
            if *v != NULL_IDENT {
                self.branches.insert(k.into(), *v);
            }
        }
    }

    #[inline]
    fn iter_specified(&self) -> impl Iterator<Item = (usize, Option<IdentId>)> + '_ {
        self.branches.iter().cloned().map(none_if_baseline).enumerate()
    }
}

// Convert to branch mask
impl BranchSpec {
    pub fn as_mask<T>(&self) -> Result<T, Error>
    where
        T: Bitmask + Default,
    {
        if self.len() > T::BITS {
            return Err(Error::BranchOutOfBounds(T::BITS, self.clone()));
        }
        let mut mask = T::default();
        for i in 0..T::BITS {
            if i >= self.len() {
                break;
            }
            if self.is_specified(i.into()) {
                mask.set(i);
            }
        }
        Ok(mask)
    }
}

#[inline]
fn none_if_baseline(id: IdentId) -> Option<IdentId> {
    if id == NULL_IDENT {
        None
    } else {
        Some(id)
    }
}
