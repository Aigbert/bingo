//! The live state shared by a call's row and its enclosing card.
use super::call_waiting;
use crate::{Interaction, Item, ItemBody, ItemStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveCall {
    Asked,
    Running,
}

/// A settled call has a closed wait and is never Running, so cards need no
/// session-wide busy guard to distinguish it from the current call.
pub fn live_call(item: &Item, interactions: &[Interaction]) -> Option<LiveCall> {
    if !matches!(item.body, ItemBody::ToolCall { .. }) {
        return None;
    }
    match item.status {
        ItemStatus::Pending | ItemStatus::Running
            if call_waiting(item)
                || interactions
                    .iter()
                    .any(|open| open.item.as_ref() == Some(&item.id)) =>
        {
            Some(LiveCall::Asked)
        }
        ItemStatus::Running => Some(LiveCall::Running),
        _ => None,
    }
}
