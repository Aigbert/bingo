//! Process primitives shared by plugins.
pub mod shell;
mod tree;
pub use tree::{SpawnError, Tree, tree};

#[cfg(test)]
mod tree_contract;
