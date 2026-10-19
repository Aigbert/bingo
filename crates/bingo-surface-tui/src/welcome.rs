//! The wordmark and the facts of this session, at the head of its transcript.

use bingo_sdk::{Driver, SessionState, SessionSummary};

/// Host answers shown at the opening, independent of session state.
#[derive(Debug, Default)]
pub struct Details {
    pub update: Option<String>,
    pub started_at_ms: Option<i64>,
}

#[derive(Clone, Copy, Default)]
pub struct Entrance<'a> {
    pub resume: Option<&'a SessionSummary>,
    pub started_at_ms: Option<i64>,
    pub selected: Option<usize>,
}

/// Whether this surface opened the session, and so has a box to say things
/// in. The start-up check asks the same question before it asks anything of
/// the network: a run with nowhere to put an answer does not go looking.
pub fn wanted(state: &SessionState) -> bool {
    state.summary.parent.is_none() && state.summary.driver == Driver::Model
}

/// Whether the transcript says nothing yet: no item, or only the records
/// rewinds left, which the prototype draws as nothing, so a rewind to the
/// first prompt brings the welcome back.
pub fn blank(state: &SessionState) -> bool {
    state
        .transcript
        .items()
        .iter()
        .all(|item| matches!(item.body, bingo_sdk::ItemBody::Rewind { .. }))
}

mod drawing;
mod embers;
pub use drawing::draw;

pub fn is_engine(memo: Option<&crate::pulled::Memo>) -> bool {
    memo.and_then(|memo| memo.answer_for(bingo_sdk::render::Component::Welcome, "Welcome"))
        .is_none_or(|view| matches!(view, bingo_sdk::View::Engine { .. }))
}

pub fn recent(ui: &crate::ui::Ui) -> Vec<SessionSummary> {
    ui.placed
        .pulled
        .answer_for(bingo_sdk::render::Component::Welcome, "Welcome")
        .and_then(|view| {
            if let bingo_sdk::View::Engine { props, .. } = view {
                serde_json::from_value(props["recent"].clone()).ok()
            } else {
                None
            }
        })
        .unwrap_or_default()
}

pub fn key(
    ui: &mut crate::ui::Ui,
    tree: &crate::tree::Tree,
    key: crossterm::event::KeyEvent,
) -> Option<Vec<crate::effect::Effect>> {
    use crate::focus::Focus;
    use crossterm::event::KeyCode;
    let chord = crate::keys::commands::chord(key);
    let listed = crate::keys::WELCOME
        .iter()
        .any(|binding| Some(binding.keys) == chord.as_deref());
    let rows = recent(ui);
    if let Focus::Welcome(index) = ui.focus {
        if rows.is_empty() {
            ui.focus = Focus::Composer;
            return None;
        }
        let index = index.min(rows.len() - 1);
        if !listed {
            ui.focus = Focus::Composer;
            return None;
        }
        match key.code {
            KeyCode::Up => ui.focus = Focus::Welcome(index.saturating_sub(1)),
            KeyCode::Down if index + 1 < rows.len() => ui.focus = Focus::Welcome(index + 1),
            KeyCode::Enter => {
                ui.focus = Focus::Composer;
                return Some(vec![crate::effect::Effect::Open(
                    bingo_sdk::SessionSelector::ById {
                        id: rows[index].id.clone(),
                    },
                )]);
            }
            KeyCode::Down | KeyCode::Esc => ui.focus = Focus::Composer,
            _ => {
                ui.focus = Focus::Composer;
                return None;
            }
        }
        return Some(vec![]);
    }
    if matches!(ui.focus, Focus::Composer)
        && listed
        && key.code == KeyCode::Up
        && ui.composer.is_empty()
        && blank(tree.viewed())
        && !rows.is_empty()
    {
        ui.focus = Focus::Welcome(rows.len() - 1);
        return Some(vec![]);
    }
    None
}

pub fn effects(
    buffer: &mut ratatui::buffer::Buffer,
    area: ratatui::layout::Rect,
    row: u16,
    at: i64,
    out: Option<i64>,
    now: crate::clock::Now,
) {
    embers::draw(buffer, area, row, at, now);
    if let Some(out) = out {
        embers::dissolve(buffer, area, out, now);
    }
}

#[cfg(test)]
mod tests {
    use crate::{focus::Focus, test_support::*};
    use crossterm::event::KeyCode;

    fn listed() -> (crate::ui::Ui, crate::tree::Tree, crate::clock::Now) {
        let mut state = state();
        state.config.kernel["render"] = serde_json::json!({"Welcome":[{}]});
        let (mut ui, now) = scene();
        let rows =
            crate::transcript::Rows::of(&state, 100, &ui.folds, &ui.decoded, &ui.linked, now);
        let request = crate::component::welcome::request(&rows).unwrap();
        let mut props = request.request.props.clone();
        props["recent"] = serde_json::json!([summary(), summary(), summary()]);
        ui.placed.pulled.request(&request);
        ui.placed.pulled.answer(
            request,
            bingo_sdk::View::Engine {
                component: bingo_sdk::render::Component::Welcome,
                props,
            },
        );
        (ui, solo(&state), now)
    }

    #[test]
    fn welcome_keys_enter_leave_resume_and_return_typing_to_the_composer() {
        let (mut ui, tree, now) = listed();
        assert!(super::key(&mut ui, &tree, crate::test_support::key(KeyCode::Up)).is_some());
        assert!(matches!(ui.focus, Focus::Welcome(2)));
        super::key(&mut ui, &tree, crate::test_support::key(KeyCode::Down));
        assert!(matches!(ui.focus, Focus::Composer));
        super::key(&mut ui, &tree, crate::test_support::key(KeyCode::Up));
        super::key(&mut ui, &tree, crate::test_support::key(KeyCode::Up));
        assert!(matches!(ui.focus, Focus::Welcome(1)));
        let effects = super::key(&mut ui, &tree, crate::test_support::key(KeyCode::Enter)).unwrap();
        assert!(matches!(
            &effects[..],
            [crate::effect::Effect::Open(
                bingo_sdk::SessionSelector::ById { .. }
            )]
        ));
        super::key(&mut ui, &tree, crate::test_support::key(KeyCode::Up));
        crate::input::on_key(&mut ui, &tree, typed('a'), now);
        assert_eq!(ui.composer.text(), "a");
        assert!(matches!(ui.focus, Focus::Composer));
    }

    #[test]
    fn a_link_answering_the_welcome_site_replaces_the_drawing() {
        let mut state = state();
        state.config.kernel["render"] = serde_json::json!({"Welcome":[{}]});
        let (mut ui, now) = scene();
        let rows =
            crate::transcript::Rows::of(&state, 100, &ui.folds, &ui.decoded, &ui.linked, now);
        let request = crate::component::welcome::request(&rows).unwrap();
        ui.placed.pulled.request(&request);
        ui.placed
            .pulled
            .answer(request, bingo_sdk::View::text("a welcome of our own"));
        let rows =
            crate::transcript::Rows::of(&state, 100, &ui.folds, &ui.decoded, &ui.linked, now)
                .components(&ui.placed.pulled);
        let drawn = crate::component::welcome::draw(&rows)
            .lines
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        assert_eq!(drawn, ["a welcome of our own"]);
        assert!(!super::is_engine(Some(&ui.placed.pulled)));
    }
}
