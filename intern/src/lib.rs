/// Interner that can only retrieve, not add new strings.
mod strs;
pub use strs::Strs;

/// Interner that checks for duplicates and only stores each unique string once.
mod packed;
pub use packed::PackedInterner;

/// Internals for mapping keys to interned strings.
mod key_to_str;
use key_to_str::KeyToStr;

/// Wrapper around interners that uses typed keys.
mod typed;
pub use typed::TypedInterner;

/// Internals for mapping interned strings to keys.
mod str_to_key;
use str_to_key::StrToKey;

type Hasher = std::hash::BuildHasherDefault<rustc_hash::FxHasher>;

/// convenience
pub type TypedStrs<T> = TypedInterner<T, Strs>;
/// If you don't want to bother to check for duplicates, just use this directly:
pub type LooseInterner<Key = u32, Idx = usize> = KeyToStr<Key, Idx>;

/// Trait for interners that can retrieve an interned string based on some `Key`.
pub trait GetStr {
    /// Key type used to fetch a string.
    type Key;

    /// Get the string associated with key `k`.
    fn get(&self, k: Self::Key) -> &str;

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
    fn intern<T: AsRef<str>>(&mut self, s: T) -> Self::Key;
}
