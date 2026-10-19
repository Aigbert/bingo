use bingo_sdk::*;
use std::collections::BTreeMap;
#[derive(Default)]
pub(crate) struct State {
    pub cursors: BTreeMap<(SessionId, String), Cursor>,
}
#[derive(Default)]
pub(crate) struct Cursor {
    pub query: String,
    pub selected: Option<crate::rows::Target>,
    pub probed: crate::probe::Probed,
}
impl Cursor {
    /// A new filter starts the cursor over at its best match.
    pub fn sync(&mut self, query: &str) {
        if self.query != query {
            self.query = query.into();
            self.selected = None;
        }
    }

    /// The cursor stays on its row while the row is listed, and otherwise
    /// falls to the first.
    pub fn settle(&mut self, rows: &[crate::rows::Row]) {
        if !rows
            .iter()
            .any(|row| Some(&row.target) == self.selected.as_ref())
        {
            self.selected = rows.first().map(|row| row.target.clone());
        }
    }
}
pub(crate) struct Cleanup(pub crate::Shared);
#[async_trait::async_trait]
impl Hook for Cleanup {
    fn id(&self) -> &str {
        "sessions.cleanup"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![HookPoint::Event],
            tool: None,
        }
    }
    async fn on_event(&self, frame: &Frame, _: &HookContext) {
        if matches!(frame.event, Event::SessionClosed { .. })
            && let Ok(mut state) = self.0.lock()
        {
            state
                .cursors
                .retain(|(session, _), _| session != &frame.session);
        }
    }
}
