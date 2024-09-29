use std::marker::PhantomData;

use anyhow::Result;

use super::{GetStr, InternStr};

#[derive(Debug)]
pub struct TypedInterner<Key, T> {
    interner: T,
    _phantom: PhantomData<Key>,
}

impl<K, T> TypedInterner<K, T> {
    pub fn new(interner: T) -> Self {
        Self {
            interner,
            _phantom: PhantomData,
        }
    }

    pub fn into_inner(self) -> T {
        self.interner
    }
}

// GetStr ///////////////////
impl<Key, T> GetStr for TypedInterner<Key, T>
where
    T: GetStr,
    Key: Into<T::Key>,
{
    type Key = Key;

    fn get(&self, k: Key) -> Result<&str> {
        self.interner.get(k.into())
    }

    fn len(&self) -> usize {
        self.interner.len()
    }

    fn str_len(&self) -> usize {
        self.interner.str_len()
    }
}

// InternStr ///////////////
impl<Key, T> InternStr for TypedInterner<Key, T>
where
    T: InternStr,
    T::Key: Into<Key>,
{
    type Key = Key;

    fn intern<U: AsRef<str>>(&mut self, s: U) -> Result<Key> {
        self.interner.intern(s).map(T::Key::into)
    }
}
