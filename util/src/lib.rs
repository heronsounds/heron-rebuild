mod bitmask;
pub use bitmask::Bitmask;

mod id_vec;
pub use id_vec::IdVec;

mod timer;
pub use timer::Timer;

#[derive(thiserror::Error, Debug)]
#[error("Filesystem path is not valid UTF-8")]
pub struct PathEncodingError;

pub type Hasher = std::hash::BuildHasherDefault<rustc_hash::FxHasher>;
pub type HashMap<K, V> = std::collections::HashMap<K, V, Hasher>;
pub type HashSet<T> = std::collections::HashSet<T, Hasher>;
