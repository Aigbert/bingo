//! What switching one session to each catalogued model would do first.
use crate::{Catalog, SessionState};

/// Marks each model entry with `compacts`: whether switching this session to
/// it compacts the chat first. It is the decision `/model` makes, so a client
/// shows it and never measures the session against `switchAt` itself.
pub trait ModelSwitch: Send + Sync {
    fn catalog(&self, catalog: Catalog, state: &SessionState) -> Catalog;
}
pub const MODEL_SWITCH: crate::ServiceKey<dyn ModelSwitch> =
    crate::ServiceKey::new("bingo.modelSwitch");
