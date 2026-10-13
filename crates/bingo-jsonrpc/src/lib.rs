//! JSON-RPC 2.0 over bounded lines.
pub mod codec;
mod lines;
mod peer;
mod pending;
pub use peer::{Handler, Peer, PeerError, Sender};

#[cfg(test)]
mod contracts;

#[cfg(test)]
mod peer_contract;
