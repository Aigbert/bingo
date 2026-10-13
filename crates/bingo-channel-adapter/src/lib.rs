//! The contract shared by channel platforms and their surface.

pub mod adapter;
pub mod conversation;
pub mod error;
pub mod limits;
pub mod question;
pub mod secret;

#[cfg(feature = "testing")]
pub mod fixtures;

/// Directory shared by channel claims and platform attachments.
pub const DIRECTORY: &str = "channels";
