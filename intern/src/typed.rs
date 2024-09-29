use std::marker::PhantomData;

use super::{GetStr, InternStr};

#[derive(Debug)]
pub struct TypedInterner<OuterKey, T> {
    interner: T,
    _phantom_outer: PhantomData<OuterKey>,
}

impl<K, T> TypedInterner<K, T> {
    pub fn new(interner: T) -> Self {
        Self {
            interner,
            _phantom_outer: PhantomData,
        }
    }

    pub fn into_inner(self) -> T {
        self.interner
    }
}

// GetStr ///////////////////
impl<OuterKey, T> GetStr for TypedInterner<OuterKey, T>
where
    T: GetStr,
    OuterKey: Into<T::Key>,
{
    type Key = OuterKey;

    fn get(&self, k: OuterKey) -> &str {
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
impl<OuterKey, T> InternStr for TypedInterner<OuterKey, T>
where
    T: InternStr,
    T::Key: Into<OuterKey>,
{
    type Key = OuterKey;

    fn intern<U: AsRef<str>>(&mut self, s: U) -> OuterKey {
        self.interner.intern(s).into()
    }
}
