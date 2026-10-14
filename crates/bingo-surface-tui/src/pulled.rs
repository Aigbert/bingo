//! Only drawn requests have a memo; late replies cannot replace newer inputs.
use bingo_sdk::{
    SessionId, SessionState, View,
    render::{RenderRequest, SurfaceName},
    slot::Shown,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    pub session: SessionId,
    pub request: RenderRequest,
    generation: u64,
}
impl Key {
    pub fn new(state: &SessionState, request: &RenderRequest, columns: u16, rows: u16) -> Self {
        let mut request = request.clone();
        request.surface = SurfaceName::Terminal;
        request.viewport.columns = columns;
        request.viewport.rows = rows;
        Self {
            session: state.summary.id.clone(),
            generation: state
                .invalidated
                .get(&request.request_id)
                .copied()
                .unwrap_or_default(),
            request,
        }
    }
    pub fn seated(
        state: &SessionState,
        request: &bingo_sdk::render::PullRequest,
        pane: &crate::pane::seat::Seated,
    ) -> Self {
        let mut key = Self::new(
            state,
            &request.on(SurfaceName::Terminal),
            pane.area.width,
            request.props["scroll"]["bodyRows"]
                .as_u64()
                .and_then(|n| u16::try_from(n).ok())
                .unwrap_or_else(|| pane.area.height.saturating_sub(1)),
        );
        key.request.viewport.place = Some(pane.place);
        key
    }
    fn same_request(&self, other: &Self) -> bool {
        self.session == other.session
            && self.request.component == other.request.component
            && self.request.request_id == other.request.request_id
    }
    fn same(&self, other: &Self) -> bool {
        self.same_request(other)
            && self.generation == other.generation
            && self.request.props == other.request.props
            && self.request.viewport.columns == other.request.viewport.columns
            && self.request.viewport.place == other.request.viewport.place
            && (!matches!(
                self.request.component,
                bingo_sdk::render::Component::Pane | bingo_sdk::render::Component::Status
            ) || self.request.viewport.rows == other.request.viewport.rows)
    }
}
#[derive(Debug)]
struct Entry {
    key: Key,
    answer: Option<(Key, View)>,
    version: u64,
}
#[derive(Debug, Default)]
pub struct Memo {
    epoch: u64,
    entries: Vec<Entry>,
    next: u64,
}
impl Memo {
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn invalidate(&mut self) {
        self.entries.clear();
        self.epoch += 1;
    }
    pub fn answer_at(&mut self, epoch: u64, key: Key, view: View) {
        if epoch == self.epoch {
            self.answer(key, view);
        }
    }
    pub fn request(&mut self, key: &Key) -> bool {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.key.same_request(key))
        {
            if entry.key.same(key) {
                return false;
            }
            entry.key = key.clone();
        } else {
            self.entries.push(Entry {
                key: key.clone(),
                answer: None,
                version: 0,
            });
        }
        true
    }
    pub fn answer(&mut self, key: Key, view: View) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.key.same(&key)) {
            self.next += 1;
            entry.version = self.next;
            entry.answer = Some((key, view));
        }
    }
    pub fn view<'a>(
        &'a self,
        shown: &'a Shown,
        state: &SessionState,
        columns: u16,
    ) -> Option<&'a View> {
        match shown {
            Shown::View(view) => Some(view),
            Shown::Pull { request } => self.display(&Key::new(
                state,
                &request.on(SurfaceName::Terminal),
                columns,
                0,
            )),
        }
    }
    pub fn get(&self, key: &Key) -> Option<&View> {
        self.entries
            .iter()
            .find(|entry| entry.key.same(key))
            .and_then(|entry| entry.answer.as_ref())
            .filter(|(answered, _)| answered.same(key))
            .map(|(_, view)| view)
    }
    /// Draw the last accepted answer, including the frame before dispatching a new key.
    pub fn display(&self, key: &Key) -> Option<&View> {
        self.get(key).or_else(|| {
            self.entries
                .iter()
                .find(|entry| entry.key.same_request(key))
                .and_then(|entry| entry.answer.as_ref())
                .map(|(_, view)| view)
        })
    }
    pub fn drawings(&self, session: &SessionId) -> impl Iterator<Item = (&Key, &View)> {
        self.entries
            .iter()
            .filter(move |entry| &entry.key.session == session)
            .filter_map(|entry| entry.answer.as_ref().map(|(key, view)| (key, view)))
    }
    /// The drawing a site shows now and whether a newer one is owed; `None`
    /// when the memo holds nothing for the site.
    pub fn current(
        &self,
        component: bingo_sdk::render::Component,
        request: &str,
        state: &SessionState,
    ) -> Option<(Option<&View>, bool)> {
        let entry = self.entries.iter().find(|entry| {
            entry.key.session == state.summary.id
                && entry.key.request.component == component
                && entry.key.request.request_id == request
        })?;
        let generation = state.invalidated.get(request).copied().unwrap_or_default();
        let waiting = entry.answer.as_ref().is_none_or(|(answered, _)| {
            !answered.same(&entry.key) || answered.generation != generation
        });
        Some((entry.answer.as_ref().map(|(_, view)| view), waiting))
    }
    pub fn version(&self, id: &str) -> u64 {
        self.entries
            .iter()
            .filter(|entry| entry.key.request.request_id == id)
            .map(|entry| entry.version)
            .max()
            .unwrap_or_default()
    }
    pub fn retain(&mut self, drawn: &[Key]) {
        self.entries.retain(|entry| {
            let answered = entry
                .answer
                .as_ref()
                .is_some_and(|(key, _)| key.same(&entry.key));
            (answered
                && !matches!(
                    entry.key.request.component,
                    bingo_sdk::render::Component::Pane | bingo_sdk::render::Component::Status
                ))
                || drawn.iter().any(|key| key.same_request(&entry.key))
        });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::render::{Component, Viewport};
    #[test]
    fn width_props_and_generation_replace_answers_but_height_does_not() {
        let mut state = crate::test_support::state();
        let request = RenderRequest {
            component: Component::AbovePrompt,
            request_id: "counter".into(),
            surface: SurfaceName::Terminal,
            props: serde_json::json!({}),
            viewport: Viewport {
                place: None,
                columns: 0,
                rows: 0,
            },
        };
        let mut memo = Memo::default();
        let first = Key::new(&state, &request, 80, 8);
        assert!(memo.request(&first));
        assert!(!memo.request(&first));
        memo.answer(first.clone(), View::text("old"));
        assert!(!memo.request(&Key::new(&state, &request, 80, 12)));
        state.invalidated.insert("counter".into(), 1);
        let next = Key::new(&state, &request, 80, 8);
        assert!(memo.request(&next));
        memo.answer(first, View::text("stale"));
        assert!(memo.get(&next).is_none());
        memo.answer(next, View::text("new"));
        assert_eq!(
            memo.view(
                &Shown::Pull {
                    request: request.clone().into()
                },
                &state,
                80
            )
            .unwrap()
            .fold(),
            "new"
        );
        assert!(memo.request(&Key::new(&state, &request, 200, 8)));
        memo.retain(&[]);
        assert!(memo.entries.is_empty());
    }
}

#[cfg(test)]
mod pane_tests {
    use super::*;
    use bingo_sdk::{
        render::{Component, Viewport},
        slot::Place,
    };
    #[test]
    fn a_pane_reasks_for_height_or_place_and_rejects_a_late_answer() {
        let state = crate::test_support::state();
        let request = RenderRequest {
            component: Component::Pane,
            request_id: "counter".into(),
            surface: SurfaceName::Terminal,
            props: serde_json::json!({}),
            viewport: Viewport {
                place: Some(Place::Side),
                columns: 64,
                rows: 20,
            },
        };
        let first = Key::new(&state, &request, 64, 20);
        let mut memo = Memo::default();
        assert!(memo.request(&first));
        memo.answer(first.clone(), View::text("old"));
        let height = Key::new(&state, &request, 64, 21);
        assert!(memo.request(&height));
        let mut moved = height.clone();
        moved.request.viewport.place = Some(Place::Above);
        assert!(memo.request(&moved));
        memo.answer(height, View::text("stale"));
        assert!(memo.get(&moved).is_none());
        memo.answer(moved.clone(), View::text("new"));
        assert_eq!(memo.get(&moved).unwrap().fold(), "new");
        assert!(!memo.request(&moved));
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use bingo_sdk::render::{Component, RenderRequest, SurfaceName, Viewport};

    #[test]
    fn an_undrawn_pane_reasks_on_reopen_but_scrolled_messages_keep_answers() {
        let state = crate::test_support::state();
        for component in [Component::Pane, Component::Status, Component::UserMessage] {
            let request = RenderRequest {
                component,
                request_id: "one".into(),
                surface: SurfaceName::Terminal,
                props: serde_json::json!({}),
                viewport: Viewport {
                    columns: 80,
                    rows: 8,
                    place: None,
                },
            };
            let key = Key::new(&state, &request, 80, 8);
            let mut memo = Memo::default();
            memo.request(&key);
            memo.answer(key.clone(), View::text("before"));
            memo.retain(&[]);
            memo.answer(key.clone(), View::text("late"));
            assert_eq!(memo.request(&key), component != Component::UserMessage);
            if component != Component::UserMessage {
                assert!(memo.get(&key).is_none());
            }
        }
    }
}

#[cfg(test)]
mod revalidation_tests {
    use super::*;
    use bingo_sdk::render::{Component, Viewport};

    #[test]
    fn invalidation_keeps_last_answer_until_current_answer_arrives() {
        let mut state = crate::test_support::state();
        let request = RenderRequest {
            component: Component::Pane,
            request_id: "art".into(),
            surface: SurfaceName::Terminal,
            props: serde_json::json!({}),
            viewport: Viewport {
                columns: 80,
                rows: 12,
                place: None,
            },
        };
        let first = Key::new(&state, &request, 80, 12);
        let mut memo = Memo::default();
        memo.request(&first);
        assert!(memo.display(&first).is_none());
        memo.answer(first.clone(), View::text("old art"));
        state.invalidated.insert("art".into(), 1);
        let next = Key::new(&state, &request, 22, 8);
        assert_eq!(memo.display(&next).unwrap().fold(), "old art");
        memo.retain(std::slice::from_ref(&next));
        memo.request(&next);
        memo.answer(first, View::text("late art"));
        assert!(memo.get(&next).is_none());
        assert_eq!(memo.display(&next).unwrap().fold(), "old art");
        let mut other = next.clone();
        other.session = SessionId::from_raw("other");
        assert!(memo.display(&other).is_none());
        other = next.clone();
        other.request.component = Component::AbovePrompt;
        assert!(memo.display(&other).is_none());
        other = next.clone();
        other.request.request_id = "other".into();
        assert!(memo.display(&other).is_none());
        memo.answer(next.clone(), View::text("new art"));
        assert_eq!(memo.display(&next).unwrap().fold(), "new art");
        memo.retain(&[]);
        assert!(memo.display(&next).is_none());
    }
}

#[cfg(test)]
mod switching_tests {
    use super::*;
    #[test]
    fn a_reply_from_before_invalidation_cannot_replace_the_new_drawing() {
        let state = crate::test_support::state();
        let request: RenderRequest = serde_json::from_value(serde_json::json!({"component":"Toast","requestId":"one","surface":"terminal","props":{},"viewport":{"columns":80,"rows":4}})).unwrap();
        let key = Key::new(&state, &request, 80, 4);
        let mut memo = Memo::default();
        let old = memo.epoch();
        memo.request(&key);
        memo.invalidate();
        memo.request(&key);
        memo.answer_at(memo.epoch(), key.clone(), View::text("new"));
        memo.answer_at(old, key.clone(), View::text("old"));
        assert_eq!(memo.get(&key).unwrap().fold(), "new");
    }
}
