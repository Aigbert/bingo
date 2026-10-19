//! Collaboration names, records and pure folds (ADR-0071).
pub mod cursor;
pub mod ear;
pub mod face;
pub mod mentions;
pub mod name;
pub mod owed;
pub mod role;
pub mod room;
pub use name::Name;
pub use room::PLUGIN;
pub mod lifecycle;
pub mod seen;
