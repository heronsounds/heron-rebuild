use anyhow::Result;
use hashbrown::hash_map::{HashMap, RawEntryMut};
use std::hash::BuildHasher;

use super::{GetStr, InternStr};

/// Internals used for keeping track of interned string ids.
/// Uses a HashMap with no value (w/ `()` as the value parameter) internally.
/// This acts as a mapping from string hash -> Key, w/o double-storing the
/// actual contents of the string.
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
    pub fn intern<T>(&mut self, s: &str, key_to_str: &mut T) -> Result<Key>
    where
        T: GetStr<Key = Key> + InternStr<Key = Key>,
    {
        // hash the string
        let hash = self.hasher.hash_one(s);
        // look up the entry for that hash;
        // the value of that entry is the key we can use to look up a string in `key_to_str`.
        let entry = self.map.raw_entry_mut().from_hash(hash, |colliding_key| {
            let already_interned = key_to_str
                .get(*colliding_key)
                .expect("This key should be guaranteed to work in the key-to-str map");
            s == already_interned
        });

        // if the entry exists, return its key.
        // if not, use `key_to_str` to assign a new key, store it in the entry,
        // and return it.
        match entry {
            RawEntryMut::Occupied(entry) => Ok(*entry.into_key()),
            RawEntryMut::Vacant(entry) => {
                let new_k = key_to_str.intern(s)?;
                entry.insert_with_hasher(hash, new_k, (), |colliding_key| {
                    let already_interned = key_to_str
                        .get(*colliding_key)
                        .expect("This key should be guaranteed to work in the key-to-str map");
                    self.hasher.hash_one(already_interned)
                });
                Ok(new_k)
            }
        }
    }
}
