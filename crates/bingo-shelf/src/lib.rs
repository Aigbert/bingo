//! Files bingo writes for a person to keep, rebuilt from their directory.

pub mod diff;
pub mod id;
mod loaded;
mod shelf;

pub use loaded::{Loaded, Unreadable};
pub use shelf::Shelf;

/// The file's grammar belongs to its owner; its stem supplies the id.
pub trait Codec {
    type Record;
    const EXT: &'static str;
    fn encode(record: &Self::Record) -> Result<String, CodecError>;
    fn decode(id: &str, text: &str) -> Result<Self::Record, CodecError>;
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct CodecError(pub String);
