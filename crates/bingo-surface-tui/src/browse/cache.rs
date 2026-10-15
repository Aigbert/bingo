use bingo_sdk::{
    SessionId, SessionState,
    component::BrowseProps,
    transcript::{self, Match, Turn},
};
use std::{cell::RefCell, sync::Arc};

#[derive(Debug, Default)]
pub struct Cache {
    data: RefCell<Projection>,
    accepted: RefCell<Option<(String, u64, Arc<BrowseProps>)>>,
}
#[derive(Debug, Default)]
struct Projection {
    key: Option<(SessionId, usize, u64)>,
    turns: Vec<Turn>,
    query: String,
    matches: Vec<Match>,
}
impl Cache {
    pub fn accepted(
        &self,
        id: &str,
        version: u64,
        read: impl FnOnce() -> Option<BrowseProps>,
    ) -> Option<Arc<BrowseProps>> {
        let mut accepted = self.accepted.borrow_mut();
        if let Some((old_id, old_version, props)) = accepted.as_ref()
            && old_id == id
            && *old_version == version
        {
            return Some(props.clone());
        }
        let props = Arc::new(read()?);
        *accepted = Some((id.into(), version, props.clone()));
        Some(props)
    }
    pub fn fill(&self, state: &SessionState, props: &mut BrowseProps) {
        let mut data = self.data.borrow_mut();
        let key = (
            state.summary.id.clone(),
            state.transcript.items().len(),
            state.transcript.history().generation,
        );
        let changed = data.key.as_ref() != Some(&key);
        if changed {
            data.turns = transcript::turns(&state.transcript);
        }
        if changed || data.query != props.query {
            data.matches = transcript::find(&state.transcript, &props.query);
            data.query.clone_from(&props.query);
            data.key = Some(key);
        }
        props.turns.clone_from(&data.turns);
        props.count = data.matches.len();
        props.current = props.current.min(props.count.saturating_sub(1));
        props.hit = data.matches.get(props.current).cloned();
        props.length = state.transcript.items().len();
        props.busy = state.busy();
    }
}
