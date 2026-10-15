use bingo_sdk::*;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Default)]
pub(crate) struct State {
    pub cursors: BTreeMap<(SessionId, String), Cursor>,
}
#[derive(Default)]
pub(crate) struct Cursor {
    pub query: String,
    pub selected: Option<crate::rows::Target>,
    pub expanded: BTreeSet<String>,
    pub searching: bool,
    pub sheet: Sheet,
    /// Why the last verb was refused, until the next key.
    pub refusal: Option<String>,
    pub probed: crate::probe::Probed,
}
#[derive(Default)]
pub(crate) enum Sheet {
    #[default]
    None,
    Loading(SessionId),
    Preview(String),
    Delete(Vec<crate::verbs::Deletion>),
    Rename {
        id: SessionId,
        text: String,
    },
}
impl Cursor {
    pub fn sync(&mut self, query: &str) {
        if self.query != query {
            self.query = query.into();
            self.selected = None;
            self.sheet = Sheet::None;
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
