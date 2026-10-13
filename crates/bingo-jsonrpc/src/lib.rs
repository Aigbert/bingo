//! JSON-RPC 2.0 over bounded lines.
pub mod codec;
mod lines;

#[cfg(test)]
mod contracts;

#[cfg(test)]
mod peer_contract;
