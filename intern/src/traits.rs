use anyhow::Result;

/// Trait for interners that can retrieve an interned string based on some `Key`.
pub trait GetStr {
    /// Key type used to fetch a string.
    type Key;

    /// Get the string associated with key `k`.
    fn get(&self, k: Self::Key) -> Result<&str>;

    /// Total number of strings interned.
    fn len(&self) -> usize;

    /// true if len is 0.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Size of interned strings.
    fn str_len(&self) -> usize;
}

/// Trait for interners that can intern a string and return a `Key`.
/// use to retrieve it later.
pub trait InternStr {
    /// Key type returned from intern, can be used to fetch string later.
    type Key;

    /// Intern string `s` and return a key that can be used to retrieve it later.
    fn intern<T: AsRef<str>>(&mut self, s: T) -> Result<Self::Key>;
}
