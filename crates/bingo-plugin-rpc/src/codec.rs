//! Plugin protocol error vocabulary.
pub use bingo_jsonrpc::codec::*;
pub const INTERNAL_ERROR: i64 = -32603;
pub const TRANSPORT_ERROR: i64 = -32010;
