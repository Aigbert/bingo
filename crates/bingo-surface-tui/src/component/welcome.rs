use crate::{transcript::Rows, welcome};
use bingo_sdk::{SessionState, component::*};
use ratatui::text::Line;

pub fn props(rows: &Rows<'_>) -> Props {
    Props::Welcome(Box::new(WelcomeProps {
        session: rows.state.summary.clone(),
        effort: crate::status::session_facts(rows.state).effort,
        modes: bingo_sdk::slot::modes(rows.state).collect(),
        update: rows.update.map(str::to_owned),
        resume: rows.entrance.resume.cloned(),
    }))
}

pub fn draw(rows: &Rows<'_>) -> Vec<Line<'static>> {
    if !welcome::wanted(rows.state) {
        return Vec::new();
    }
    let props = props(rows);
    super::resolve(
        rows.state,
        rows.pulled,
        "Welcome",
        &props,
        rows.width,
        |changed, width| {
            let Props::Welcome(facts) = changed else {
                return Vec::new();
            };
            let entrance = welcome::Entrance {
                height: rows.entrance.height,
                resume: facts.resume.as_ref(),
            };
            if changed == &props {
                return welcome::draw(
                    rows.state,
                    width,
                    entrance,
                    facts.update.as_deref(),
                    rows.now.wall,
                );
            }
            let state = changed_state(facts);
            welcome::draw(
                &state,
                width,
                entrance,
                facts.update.as_deref(),
                rows.now.wall,
            )
        },
    )
}

pub fn request(rows: &Rows<'_>) -> Option<crate::pulled::Key> {
    welcome::wanted(rows.state)
        .then(|| super::key(rows.state, "Welcome", &props(rows), rows.width as u16, 0))
        .flatten()
}

fn changed_state(facts: &WelcomeProps) -> SessionState {
    let mut state = SessionState::new(facts.session.clone());
    state.config.kernel["thinking"] = serde_json::to_value(facts.effort).unwrap_or_default();
    state.signals.entry("welcome".into()).or_default().insert(
        "modes".into(),
        bingo_sdk::slot::with(
            serde_json::json!({}),
            facts
                .modes
                .iter()
                .map(|current| bingo_sdk::slot::Slot::Mode {
                    current: current.clone(),
                    modes: Vec::new(),
                }),
        )
        .into(),
    );
    state
}
