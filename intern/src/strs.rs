use super::{GetStr, KeyToStr};
use anyhow::Result;

/// An interner that has been frozen and does not allow adding new strings.
/// We freeze it by discarding the string-to-key mapping, so we can't look up
/// ids for strings anymore either.
#[derive(Debug)]
pub struct Strs<Key = u32, Idx = usize> {
    key_to_str: KeyToStr<Key, Idx>,
}

impl<Key, Idx> Strs<Key, Idx> {
    pub fn new(key_to_str: KeyToStr<Key, Idx>) -> Self {
        Self { key_to_str }
    }
}

// GetStr ///////////////////
impl<Key, Idx> GetStr for Strs<Key, Idx>
where
    KeyToStr<Key, Idx>: GetStr<Key = Key>,
{
    type Key = Key;

    fn get(&self, k: Key) -> Result<&str> {
        self.key_to_str.get(k)
    }

    fn len(&self) -> usize {
        self.key_to_str.len()
    }

    fn str_len(&self) -> usize {
        self.key_to_str.str_len()
    }
}
