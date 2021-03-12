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
        Self {
            key_to_str: Vec::with_capacity(cap),
            strings: String::with_capacity(cap * avg_len),
            _phantom: PhantomData,
        }
    }
}

// GetStr ////////////////////
impl<Key, Idx> GetStr<Key> for KeyToStr<Key, Idx>
where
    Key: TryInto<usize> + TryFrom<usize>,
    Idx: Into<usize> + From<usize> + Copy,
{
    fn get(&self, k: Key) -> &str {
        let (start, end) = self.get_start_and_end(to_usize(k));
        &self.strings[start..end]
    }

    fn len(&self) -> usize {
        self.key_to_str.len()
    }
}

// InternStr ///////////////////
impl<Key, Idx> InternStr<Key> for KeyToStr<Key, Idx>
where
    Key: TryInto<usize> + TryFrom<usize>,
    Idx: Into<usize> + From<usize> + Copy,
{
    fn intern<T: AsRef<str>>(&mut self, s: T) -> Key {
        let s = s.as_ref();
        let start = self.strings.len();
        let k = from_usize(self.key_to_str.len());

        self.key_to_str.push(start.into());
        self.strings.push_str(s);

        k
    }
}

impl<Key, Idx> KeyToStr<Key, Idx>
where
    Idx: Into<usize> + Copy,
{
    fn get_start_and_end(&self, k: usize) -> (usize, usize) {
        let start = self.key_to_str[k].into();
        let end = if k == self.key_to_str.len() - 1 {
            self.strings.len()
        } else {
            self.key_to_str[k + 1].into()
        };
        (start, end)
    }
}

impl<Key, Idx> From<KeyToStr<Key, Idx>> for Strs<Key, Idx> {
    fn from(val: KeyToStr<Key, Idx>) -> Self {
        Self::new(val)
    }
}

fn to_usize<T: TryInto<usize>>(x: T) -> usize {
    x.try_into().ok().unwrap()
}

fn from_usize<T: TryFrom<usize>>(x: usize) -> T {
    T::try_from(x).ok().unwrap()
}
