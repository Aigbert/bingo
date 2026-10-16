//! The session tree folded from one attachment's frames and gateway events.

use std::collections::{BTreeMap, BTreeSet};

use crate::{Applied, CloseReason, Event, Frame, GatewayEvent, SessionId, SessionState};

/// One root and the descendants announced on its attachment's stream.
#[derive(Clone, Debug)]
pub struct TreeState {
    root: SessionState,
    children: BTreeMap<SessionId, SessionState>,
}

impl TreeState {
    pub fn new(root: SessionState) -> Self {
        Self {
            root,
            children: BTreeMap::new(),
        }
    }

    /// Unknown sessions need a summary before any other frame can apply.
    pub fn apply(&mut self, frame: &Frame) -> Applied {
        if frame.session == self.root.summary.id {
            return self.root.apply(frame);
        }
        if !self.children.contains_key(&frame.session) {
            let Event::SessionUpdated { summary } = &frame.event else {
                return Applied::Stale;
            };
            self.children
                .insert(frame.session.clone(), SessionState::new(summary.clone()));
        }
        let Some(state) = self.children.get_mut(&frame.session) else {
            return Applied::Stale;
        };
        let applied = state.apply(frame);
        if applied != Applied::Stale
            && matches!(
                frame.event,
                Event::SessionClosed {
                    reason: CloseReason::Deleted
                }
            )
        {
            self.remove(&frame.session);
        }
        applied
    }

    /// Remove a stored child and its subtree; the attachment owns its root.
    pub fn removed(&mut self, event: &GatewayEvent) -> Vec<SessionId> {
        match event {
            GatewayEvent::SessionRemoved { session } => self.remove(session),
            _ => Vec::new(),
        }
    }

    fn remove(&mut self, session: &SessionId) -> Vec<SessionId> {
        if session == &self.root.summary.id {
            return Vec::new();
        }
        let mut reached = BTreeSet::from([session.clone()]);
        loop {
            let before = reached.len();
            for child in self.children.values() {
                if child
                    .summary
                    .parent
                    .as_ref()
                    .is_some_and(|p| reached.contains(&p.session))
                {
                    reached.insert(child.summary.id.clone());
                }
            }
            if reached.len() == before {
                break;
            }
        }
        reached
            .into_iter()
            .filter(|id| self.children.remove(id).is_some())
            .collect()
    }

    pub fn root(&self) -> &SessionState {
        &self.root
    }

    pub fn get(&self, id: &SessionId) -> Option<&SessionState> {
        if id == &self.root.summary.id {
            Some(&self.root)
        } else {
            self.children.get(id)
        }
    }

    /// Root first, followed by descendants in session id order.
    pub fn sessions(&self) -> impl Iterator<Item = &SessionState> {
        std::iter::once(&self.root).chain(self.children.values())
    }

    pub fn children_of<'a>(&'a self, id: &'a SessionId) -> impl Iterator<Item = &'a SessionState> {
        self.children
            .values()
            .filter(move |s| s.summary.parent.as_ref().is_some_and(|p| &p.session == id))
    }

    /// Preserve the client's existing read marker without mutable tree access.
    pub fn mark_read(&mut self, id: &SessionId) {
        if id == &self.root.summary.id {
            self.root.mark_read();
        } else if let Some(child) = self.children.get_mut(id) {
            child.mark_read();
        }
    }
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
        for reason in [
            CloseReason::Shutdown,
            CloseReason::Client,
            CloseReason::Error {
                message: "failed".into(),
            },
        ] {
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

    fn step(tree: &mut TreeState, (kind, n, parent): (u8, u8, u8), seq: u64) {
        match kind {
            0 => {
                tree.apply(&announce(n, parent, seq));
            }
            1 => {
                tree.apply(&close(n, seq, CloseReason::Shutdown));
            }
            2 => {
                tree.apply(&close(n, seq, CloseReason::Deleted));
            }
            _ => {
                tree.removed(&removed(n));
            }
        }
    }

    fn snapshot(tree: &TreeState) -> serde_json::Value {
        serde_json::to_value(tree.sessions().collect::<Vec<_>>()).unwrap()
    }

    fn prune(parents: &mut BTreeMap<u8, u8>, n: u8) {
        let mut pending = vec![n];
        while let Some(parent) = pending.pop() {
            let children: Vec<_> = parents
                .iter()
                .filter_map(|(&child, &p)| (p == parent).then_some(child))
                .collect();
            for child in children {
                parents.remove(&child);
                pending.push(child);
            }
        }
        parents.remove(&n);
    }

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(256))]
        #[test]
        fn tree_matches_replay_and_removal_oracle(
            codes in proptest::collection::vec((0u8..4, 1u8..8, 0u8..8), 0..80)
        ) {
            let mut incremental = tree();
            let mut parents = BTreeMap::new();
            for (at, &(kind, n, parent)) in codes.iter().enumerate() {
                let known = parents.contains_key(&n);
                step(&mut incremental, (kind, n, parent), at as u64 + 1);
                match kind {
                    0 => { parents.insert(n, parent); }
                    2 if known => prune(&mut parents, n),
                    3 => prune(&mut parents, n),
                    _ => {}
                }
                let expected: Vec<_> = std::iter::once(id(0)).chain(parents.keys().map(|&n| id(n))).collect();
                let actual: Vec<_> = incremental.sessions().map(|s| s.summary.id.clone()).collect();
                proptest::prop_assert_eq!(actual, expected);
                let mut replay = tree();
                for (index, &code) in codes[..=at].iter().enumerate() {
                    step(&mut replay, code, index as u64 + 1);
                }
                proptest::prop_assert_eq!(snapshot(&incremental), snapshot(&replay));
            }
        }
    }
}
