//! The two slash commands over the store (ADR-0019 §6, §8): the table of
//! everything that fires later with the person's own verbs over it, and the
//! one wake the model set on this session.

pub(crate) mod consent;
mod schedule;
mod wake;

pub use schedule::ScheduleCommand;
pub use wake::WakeCommand;
