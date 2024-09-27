use std::marker::PhantomData;

use super::{GetStr, InternStr, Strs};

/// Internals used by all of our interners.
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
impl<Key, Idx> GetStr<Key> for KeyToStr<Key, Idx>
where
    Key: TryInto<usize>,
    Idx: TryInto<usize> + Copy,
{
    fn get(&self, k: Key) -> &str {
        let k = into_usize(k);
        let start = into_usize(self.key_to_str[k]);
        let end = if k == self.key_to_str.len() - 1 {
            self.strings.len()
        } else {
            into_usize(self.key_to_str[k + 1])
        };

        &self.strings[start..end]
    }

    fn len(&self) -> usize {
        self.key_to_str.len()
    }

    fn str_len(&self) -> usize {
        self.strings.len()
    }
}

// InternStr ///////////////////
impl<Key, Idx> InternStr<Key> for KeyToStr<Key, Idx>
where
    Key: TryFrom<usize>,
    Idx: TryFrom<usize>,
{
    fn intern<T: AsRef<str>>(&mut self, s: T) -> Key {
        let s = s.as_ref();
        let start = self.strings.len();
        let k = from_usize(self.key_to_str.len());

        self.key_to_str.push(from_usize(start));
        self.strings.push_str(s);

        k
    }
}

impl<Key, Idx> From<KeyToStr<Key, Idx>> for Strs<Key, Idx> {
    fn from(val: KeyToStr<Key, Idx>) -> Self {
        Self::new(val)
    }
}

fn into_usize<T: TryInto<usize>>(x: T) -> usize {
    x.try_into()
        .ok()
        .expect("Invalid conversion from interner key or index to usize")
}

fn from_usize<T: TryFrom<usize>>(x: usize) -> T {
    T::try_from(x)
        .ok()
        .expect("Invalid conversion from usize to interner key or index")
}
