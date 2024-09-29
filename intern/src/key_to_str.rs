use std::marker::PhantomData;

use anyhow::Result;

use super::{Error, GetStr, InternStr, Strs};

/// Internals used by all of our interners.
/// `Key` is the id of a substring; `Idx` is an index into the string storage
/// where that substring is located.
#[derive(Debug)]
pub struct KeyToStr<Key = u32, Idx = usize> {
    key_to_str: Vec<Idx>,
    strings: String,
    _phantom: PhantomData<Key>,
}

impl<Key, Idx> KeyToStr<Key, Idx> {
    pub fn with_capacity_and_avg_len(cap: usize, avg_len: usize) -> Self {
        Self::with_capacity_and_str_len(cap, cap * avg_len)
    }

    pub fn with_capacity_and_str_len(cap: usize, str_len: usize) -> Self {
        Self {
            key_to_str: Vec::with_capacity(cap),
            strings: String::with_capacity(str_len),
            _phantom: PhantomData,
        }
    }
}

// GetStr ////////////////////
impl<Key, Idx> GetStr for KeyToStr<Key, Idx>
where
    Key: TryInto<usize>,
    Idx: TryInto<usize> + Copy,
    anyhow::Error: From<Key::Error> + From<Idx::Error>,
{
    type Key = Key;

    fn get(&self, k: Key) -> Result<&str> {
        let k = k.try_into()?;

        let start = self.key_to_str.get(k).copied().ok_or(Error::KeyNotFound(k))?.try_into()?;

        let end = if k == self.key_to_str.len() - 1 {
            self.strings.len()
        } else {
            self.key_to_str[k + 1].try_into()?
        };

        Ok(&self.strings[start..end])
    }

    fn len(&self) -> usize {
        self.key_to_str.len()
    }

    fn str_len(&self) -> usize {
        self.strings.len()
    }
}

// InternStr ///////////////////
impl<Key, Idx> InternStr for KeyToStr<Key, Idx>
where
    Key: TryFrom<usize>,
    Idx: TryFrom<usize>,
    anyhow::Error: From<Key::Error> + From<Idx::Error>,
{
    type Key = Key;

    // Note that this impl *does not* check for duplicates;
    // if given a duplicate string it will just re-intern it and return a new key.
    // Checking for duplicates should be done by a wrapper *before* calling this fn.
    // Specifically, see `StrToKey`.
    fn intern<T: AsRef<str>>(&mut self, s: T) -> Result<Key> {
        let str_len = self.strings.len();
        let start = Idx::try_from(str_len).map_err(|_| Error::StringIndexOutOfBounds(str_len))?;

        let keys_len = self.key_to_str.len();
        let k = Key::try_from(keys_len).map_err(|_| Error::OutOfKeySpace(keys_len))?;

        self.key_to_str.push(start);
        self.strings.push_str(s.as_ref());

        Ok(k)
    }
}

impl<Key, Idx> From<KeyToStr<Key, Idx>> for Strs<Key, Idx> {
    fn from(val: KeyToStr<Key, Idx>) -> Self {
        Self::new(val)
    }
}
