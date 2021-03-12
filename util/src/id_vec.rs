use std::marker::PhantomData;

/// Vec wrapper that uses typed indexes.
#[derive(Debug, Default, Hash, PartialEq, Eq, Clone)]
pub struct IdVec<K, V> {
    vec: Vec<V>,
    _phantom: PhantomData<K>,
}

impl<K, V> IdVec<K, V> {
    /// Create a new `IdVec` backed by the given vec.
    fn new(vec: Vec<V>) -> Self {
        Self {
            vec,
            _phantom: PhantomData,
        }
    }

    /// Create a new `IdVec` with the given capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self::new(Vec::with_capacity(cap))
    }

    /// Get the current length
    #[inline]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    /// True if len == 0
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    /// Iterate through immutable references to values
    pub fn iter(&self) -> std::slice::Iter<'_, V> {
        self.vec.iter()
    }

    /// Iterate through mutable references to values
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, V> {
        self.vec.iter_mut()
    }
}

impl<K, V: Clone> IdVec<K, V> {
    /// Create a new `IdVec`, filled with `len` copies of `val`.
    pub fn fill(val: V, len: usize) -> Self {
        Self::new(vec![val; len])
    }
}

impl<K: Into<usize>, V> IdVec<K, V> {
    /// Get the value with id `k`.
    #[inline]
    pub fn get(&self, k: K) -> &V {
        &self.vec[k.into()]
    }

    /// Get a mutable reference to value with id `k`.
    #[inline]
    pub fn get_mut(&mut self, k: K) -> &mut V {
        &mut self.vec[k.into()]
    }
}

impl<K: From<usize>, V> IdVec<K, V> {
    /// Push `v` into the underlying vec, and return an id that can be used to retrieve it later.
    #[inline]
    pub fn push(&mut self, v: V) -> K {
        let id = self.vec.len().into();
        self.vec.push(v);
        id
    }
}

impl<K: Into<usize>, V: Default + Clone> IdVec<K, V> {
    /// Insert value `v` at position `k`.
    /// The underlying vec will be extended if `k` is beyond its current capacity,
    /// and new entries will be filled with a default value.
    pub fn insert(&mut self, k: K, v: V) {
        let k = k.into();
        let len = self.vec.len();
        if k >= len {
            self.vec.reserve(k + 1);
            self.vec.append(&mut vec![V::default(); k + 1 - len]);
        }
        self.vec[k] = v;
    }
}
