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

/// Core traits this crate is built on:
mod traits;
pub use traits::{GetStr, InternStr};

type Hasher = std::hash::BuildHasherDefault<rustc_hash::FxHasher>;

/// convenience
pub type TypedStrs<T> = TypedInterner<T, Strs>;

/// If you don't want to bother to check for duplicates with PackedInterner,
/// just use this directly:
pub type LooseInterner<Key = u32, Idx = usize> = KeyToStr<Key, Idx>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Key not found in the interner mapping: {0}")]
    KeyNotFound(usize),
    #[error("Out of string index space; {0} is greater than the maximum string index")]
    StringIndexOutOfBounds(usize),
    #[error("Out of interner key space; {0} is greater than the maximum key value")]
    OutOfKeySpace(usize),
}
