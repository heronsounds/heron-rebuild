/// Interner that can only retrieve, not add new strings.
mod strs;
pub use strs::Strs;

/// Interner that doesn't check for duplicates.
mod loose;
pub use loose::LooseInterner;

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

/// Trait for interners that can retrieve an interned string based on some key `K`.
pub trait GetStr<K> {
    /// Get the string associated with key `k`.
    fn get(&self, k: K) -> &str;

    /// Total number of strings interned.
    fn len(&self) -> usize;

    /// true if len is 0.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Size of interned strings.
    fn str_len(&self) -> usize;
}

/// Trait for interners that can intern a string and return a key `K`
/// use to retrieve it later.
pub trait InternStr<K> {
    /// Intern string `s` and return a key that can be used to retrieve it later.
    fn intern<T: AsRef<str>>(&mut self, s: T) -> K;
}
