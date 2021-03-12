use traverse::RealTaskKey;
use util::{HashMap, Hasher, IdVec};
use workflow::RealTaskId;

use super::ActualTaskId;

/// Keeps track of duplicate task/branch pairs, and assigns each unique pair
/// an id for use in the `ActionResolver`'s metadata mappings.
pub struct Deduper {
    /// map task ids to *deduped* task ids used in `should_run` and `outputs`:
    id_map: IdVec<RealTaskId, ActualTaskId>,
    /// keep track of task/branch pairs we've already encountered:
    seen_tasks: HashMap<RealTaskKey, ActualTaskId>,
    /// number of unique tasks we've seen so far:
    dedupe_count: ActualTaskId,
}

impl Deduper {
    /// Create a new `Deduper` with the given capacity,
    /// representing the number of non-deduped tasks in a traversal.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            id_map: IdVec::with_capacity(cap),
            seen_tasks: HashMap::with_capacity_and_hasher(cap, Hasher::default()),
            dedupe_count: 0,
        }
    }

    /// true if we've seen this key before.
    /// either way, adds a new id to the id map.
    pub fn is_dupe(&mut self, key: &RealTaskKey) -> bool {
        if let Some(id) = self.seen_tasks.get(key) {
            self.id_map.push(*id);
            true
        } else {
            self.seen_tasks.insert(key.clone(), self.dedupe_count);
            self.id_map.push(self.dedupe_count);
            self.dedupe_count += 1;
            false
        }
    }

    /// get usable id into the deduped task vecs from the id map.
    pub fn get_actual_task_id(&self, id: RealTaskId) -> ActualTaskId {
        *self.id_map.get(id)
    }
}
