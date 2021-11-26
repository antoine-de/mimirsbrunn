pub mod adapters;
pub mod domain;
pub mod utils;

// Since common doesn't make a lot of sense outside of this repository's
// context, it makes sense to re-export it here instead of publishing it as
// its own package.
pub use common;

#[cfg(test)]
mod tests;
