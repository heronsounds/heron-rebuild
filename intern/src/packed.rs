use std::hash::BuildHasher;

use super::{GetStr, InternStr, KeyToStr, StrToKey, Strs};

/// Interner that checks for duplicates and will only intern a given string once.
/// Using the lasso/rodeo hack for packed maps.
/// First param ("Key") is the id and must be big enough to fit total items;
/// Second param ("Idx") is an index into the string and must be greater than string len.
#[derive(Debug)]
pub struct PackedInterner<Key = u32, Idx = usize, H = crate::Hasher> {
    str_to_key: StrToKey<Key, H>,
    key_to_str: KeyToStr<Key, Idx>,
}

impl<Key, Idx> PackedInterner<Key, Idx, crate::Hasher> {
    pub fn with_capacity_and_avg_len(cap: usize, avg_len: usize) -> Self {
        Self {
            str_to_key: StrToKey::with_capacity(cap),
            key_to_str: KeyToStr::with_capacity_and_avg_len(cap, avg_len),
        }
    }

    pub fn with_capacity_and_str_len(cap: usize, str_len: usize) -> Self {
        Self {
            str_to_key: StrToKey::with_capacity(cap),
            key_to_str: KeyToStr::with_capacity_and_str_len(cap, str_len),
        }
    }
}

// GetStr /////////////////////
impl<Key, Idx, H: BuildHasher> GetStr<Key> for PackedInterner<Key, Idx, H>
where
    KeyToStr<Key, Idx>: GetStr<Key>,
{
    fn get(&self, k: Key) -> &str {
        self.key_to_str.get(k)
    }

    fn len(&self) -> usize {
        self.key_to_str.len()
    }

    fn str_len(&self) -> usize {
        self.key_to_str.str_len()
    }
}

// InternStr ///////////////////
impl<Key, Idx, H: BuildHasher> InternStr<Key> for PackedInterner<Key, Idx, H>
where
    Key: Copy,
    KeyToStr<Key, Idx>: GetStr<Key> + InternStr<Key>,
{
    fn intern<T: AsRef<str>>(&mut self, s: T) -> Key {
        let s = s.as_ref();
        self.str_to_key.intern(s, &mut self.key_to_str)
    }
}

impl<Key, Idx, H> From<PackedInterner<Key, Idx, H>> for Strs<Key, Idx> {
    fn from(val: PackedInterner<Key, Idx, H>) -> Self {
        val.key_to_str.into()
    }
}
