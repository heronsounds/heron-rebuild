#[macro_use]
mod macros;
mod parse;
pub use parse::parse;
pub mod ast;
mod bash;

type Hasher = std::hash::BuildHasherDefault<rustc_hash::FxHasher>;
type HashSet<T> = std::collections::HashSet<T, Hasher>;
