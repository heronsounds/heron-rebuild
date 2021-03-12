use super::{GetStr, InternStr, KeyToStr, Strs};

/// Interner that doesn't bother to check for duplicates.
/// We use it for literals, since they're unlikely to repeat often.
#[derive(Debug)]
pub struct LooseInterner<Key = u32, Idx = usize> {
    key_to_str: KeyToStr<Key, Idx>,
}

impl<Key, Idx> LooseInterner<Key, Idx> {
    pub fn with_capacity_and_avg_len(cap: usize, avg_len: usize) -> Self {
        Self {
            key_to_str: KeyToStr::with_capacity_and_avg_len(cap, avg_len),
        }
    }
}

// InternStr ///////////////////
impl<Key, Idx> InternStr<Key> for LooseInterner<Key, Idx>
where
    KeyToStr<Key, Idx>: InternStr<Key>,
{
    fn intern<T: AsRef<str>>(&mut self, s: T) -> Key {
        self.key_to_str.intern(s)
    }
}

// GetStr /////////////////////
impl<Key, Idx> GetStr<Key> for LooseInterner<Key, Idx>
where
    KeyToStr<Key, Idx>: GetStr<Key>,
{
    fn get(&self, k: Key) -> &str {
        self.key_to_str.get(k)
    }

    fn len(&self) -> usize {
        self.key_to_str.len()
    }
}

impl<Key, Idx> From<LooseInterner<Key, Idx>> for Strs<Key, Idx> {
    fn from(val: LooseInterner<Key, Idx>) -> Self {
        val.key_to_str.into()
    }
}
