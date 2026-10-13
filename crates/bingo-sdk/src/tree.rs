//! The session tree folded from one attachment's frames and gateway events.

use crate::{Applied, Frame, GatewayEvent, SessionId, SessionState};

pub struct TreeState {
    root: SessionState,
}

impl TreeState {
    pub fn new(root: SessionState) -> Self {
        Self { root }
    }
    pub fn apply(&mut self, _frame: &Frame) -> Applied {
        Applied::Stale
    }
    pub fn removed(&mut self, _event: &GatewayEvent) -> Vec<SessionId> {
        Vec::new()
    }
    pub fn root(&self) -> &SessionState {
        &self.root
    }
    pub fn get(&self, id: &SessionId) -> Option<&SessionState> {
        (id == &self.root.summary.id).then_some(&self.root)
    }
    pub fn sessions(&self) -> impl Iterator<Item = &SessionState> {
        std::iter::once(&self.root)
    }
    pub fn children_of<'a>(&'a self, id: &'a SessionId) -> impl Iterator<Item = &'a SessionState> {
        self.sessions()
            .filter(move |s| s.summary.parent.as_ref().is_some_and(|p| &p.session == id))
    }
    pub fn mark_read(&mut self, _id: &SessionId) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CloseReason, Event, Seq, SessionSummary, Usage};

    fn id(n: u8) -> SessionId {
        SessionId::from_raw(format!("s{n}"))
    }
    fn summary(n: u8, parent: Option<u8>) -> SessionSummary {
        serde_json::from_value(serde_json::json!({
            "id": id(n), "cwd": "/tmp", "driver": "model",
            "parent": parent.map(|p| serde_json::json!({"session": id(p)})),
            "createdAt": "1970-01-01T00:00:00Z", "updatedAt": "1970-01-01T00:00:00Z",
            "usage": Usage::default(), "busy": false
        }))
        .unwrap()
    }
    fn tree() -> TreeState {
        TreeState::new(SessionState::new(summary(0, None)))
    }
    fn frame(n: u8, seq: u64, event: Event) -> Frame {
        Frame {
            session: id(n),
            seq: Seq(seq),
            ts: jiff::Timestamp::UNIX_EPOCH,
            cause: None,
            event,
        }
    }
    fn announce(n: u8, parent: u8, seq: u64) -> Frame {
        frame(
            n,
            seq,
            Event::SessionUpdated {
                summary: summary(n, Some(parent)),
            },
        )
    }
    fn close(n: u8, seq: u64, reason: CloseReason) -> Frame {
        frame(n, seq, Event::SessionClosed { reason })
    }
    fn removed(n: u8) -> GatewayEvent {
        GatewayEvent::SessionRemoved { session: id(n) }
    }

    #[test]
    fn a_child_announces_then_folds_and_unknown_frames_are_stale() {
        let mut tree = tree();
        assert_eq!(
            tree.apply(&close(1, 1, CloseReason::Shutdown)),
            Applied::Stale
        );
        assert_eq!(tree.sessions().count(), 1);
        assert_eq!(tree.apply(&announce(1, 0, 1)), Applied::Session);
        tree.apply(&close(1, 2, CloseReason::Shutdown));
        assert!(tree.get(&id(1)).unwrap().closed);
        assert_eq!(tree.children_of(&id(0)).count(), 1);
    }

    #[test]
    fn deletion_removes_descendants_and_late_frames_need_a_new_announcement() {
        let mut tree = tree();
        tree.apply(&announce(2, 1, 1));
        tree.apply(&announce(1, 0, 1));
        assert_eq!(
            tree.sessions()
                .map(|s| s.summary.id.clone())
                .collect::<Vec<_>>(),
            vec![id(0), id(1), id(2)]
        );
        assert_eq!(
            tree.apply(&close(1, 2, CloseReason::Deleted)),
            Applied::Session
        );
        assert_eq!(tree.sessions().count(), 1);
        assert_eq!(
            tree.apply(&close(2, 3, CloseReason::Shutdown)),
            Applied::Stale
        );
        tree.apply(&announce(2, 0, 4));
        assert!(tree.get(&id(2)).is_some());
    }

    #[test]
    fn other_closes_preserve_the_state_for_reopening() {
        for reason in [CloseReason::Shutdown, CloseReason::Client] {
            let mut tree = tree();
            tree.apply(&announce(1, 0, 1));
            tree.apply(&frame(
                1,
                2,
                Event::Extension {
                    plugin: "test".into(),
                    kind: "kept".into(),
                    payload: serde_json::json!(42),
                },
            ));
            tree.apply(&close(1, 3, reason));
            tree.apply(&announce(1, 0, 4));
            let child = tree.get(&id(1)).unwrap();
            assert!(!child.closed);
            assert_eq!(child.extensions["test"]["kept"], 42);
        }
    }

    #[test]
    fn gateway_removal_returns_the_subtree_and_preserves_the_root() {
        let mut tree = tree();
        tree.apply(&announce(2, 1, 1));
        tree.apply(&announce(1, 0, 1));
        assert!(tree.removed(&removed(0)).is_empty());
        assert_eq!(tree.removed(&removed(1)), vec![id(1), id(2)]);
        assert_eq!(tree.sessions().count(), 1);
        assert!(tree.removed(&removed(1)).is_empty());
    }

    #[test]
    fn stale_deletion_does_not_remove_a_child() {
        let mut tree = tree();
        tree.apply(&announce(1, 0, 3));
        assert_eq!(
            tree.apply(&close(1, 2, CloseReason::Deleted)),
            Applied::Stale
        );
        assert!(tree.get(&id(1)).is_some());
    }
}
