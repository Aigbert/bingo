//! Process primitives shared by plugins.
pub mod shell;

/// Start a child whose pipes and lifetime belong to its whole tree.
/// The tree exposes take_stdin/stdout/stderr, id, wait, start_kill and
/// terminate(grace); dropping an unreaped tree ends all its processes.
#[cfg(test)]
mod tree_contract;
