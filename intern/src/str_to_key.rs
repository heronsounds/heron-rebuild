use hashbrown::hash_map::{HashMap, RawEntryMut};
use std::hash::BuildHasher;

use super::{GetStr, InternStr};

/// Internals used for keeping track of interned string ids.
#[derive(Debug)]
pub struct StrToKey<Key = u32, H = crate::Hasher> {
    map: HashMap<Key, (), ()>,
    hasher: H,
}

impl<Key, H: Default> StrToKey<Key, H> {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            map: HashMap::with_capacity_and_hasher(cap, ()),
            hasher: H::default(),
        }
    }
}

impl<Key: Copy, H: BuildHasher> StrToKey<Key, H> {
    pub fn intern<T>(&mut self, s: &str, strs: &mut T) -> Key
    where
        T: GetStr<Key> + InternStr<Key>,
    {
        let hash = self.hasher.hash_one(s);
        let entry = self.map.raw_entry_mut().from_hash(hash, |key| {
            let interned = strs.get(*key);
            s == interned
        });

        match entry {
            RawEntryMut::Occupied(entry) => *entry.into_key(),
            RawEntryMut::Vacant(entry) => {
                let new_k = strs.intern(s);
                entry.insert_with_hasher(hash, new_k, (), |key| {
                    let interned_str = strs.get(*key);
                    self.hasher.hash_one(interned_str)
                });
                new_k
            }
        }
    }
}
