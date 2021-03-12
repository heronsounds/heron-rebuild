use std::marker::PhantomData;

use super::{GetStr, InternStr};

#[derive(Debug)]
pub struct TypedInterner<OuterKey, T, InnerKey = u32> {
    interner: T,
    _phantom_outer: PhantomData<OuterKey>,
    _phantom_inner: PhantomData<InnerKey>,
}

impl<K, T, L> TypedInterner<K, T, L> {
    pub fn new(interner: T) -> Self {
        Self {
            interner,
            _phantom_outer: PhantomData,
            _phantom_inner: PhantomData,
        }
    }

    pub fn into_inner(self) -> T {
        self.interner
    }
}

// GetStr ///////////////////
impl<OuterKey, T, InnerKey> GetStr<OuterKey> for TypedInterner<OuterKey, T, InnerKey>
where
    OuterKey: Into<InnerKey>,
    T: GetStr<InnerKey>,
{
    fn get(&self, k: OuterKey) -> &str {
        self.interner.get(k.into())
    }

    fn len(&self) -> usize {
        self.interner.len()
    }
}

// InternStr ///////////////
impl<OuterKey, T, InnerKey> InternStr<OuterKey> for TypedInterner<OuterKey, T, InnerKey>
where
    InnerKey: Into<OuterKey>,
    T: InternStr<InnerKey>,
{
    fn intern<U: AsRef<str>>(&mut self, s: U) -> OuterKey {
        self.interner.intern(s).into()
    }
}
