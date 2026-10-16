//! Toast expiry follows its published wall-clock deadline.
use crate::{clock::Now, theme, views};
use bingo_sdk::{SessionState, slot::Slot};
use ratatui::text::{Line, Span};
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};
#[derive(Debug, Default)]
pub struct State {
    seen: BTreeSet<String>,
}
struct Toast {
    key: String,
    text: String,
    until: jiff::Timestamp,
    alert: bool,
}
fn placed(state: &SessionState) -> Vec<Toast> {
    bingo_sdk::slot::placed(state)
        .into_iter()
        .filter_map(|placed| {
            if let Slot::Toast { text, until, alert } = placed.slot {
                Some(Toast {
                    key: format!(
                        "{}/{}/{}/{until}/{text}/{alert}",
                        state.summary.id, placed.plugin, placed.kind
                    ),
                    text,
                    until,
                    alert,
                })
            } else {
                None
            }
        })
        .collect()
}
impl State {
    /// Returns whether a newly seen alert should notify the terminal.
    pub fn sync(&mut self, state: &SessionState, now: Now) -> bool {
        let toasts = placed(state);
        self.seen
            .retain(|key| toasts.iter().any(|toast| &toast.key == key));
        let mut alert = false;
        for toast in toasts {
            let unseen = self.seen.insert(toast.key);
            alert |= unseen && toast.alert && toast.until > now.wall;
        }
        alert
    }
    fn remaining(&self, toast: &Toast, now: Now) -> Option<Duration> {
        (toast.until > now.wall).then(|| toast.until.duration_since(now.wall).unsigned_abs())
    }
    pub fn components(
        &self,
        now: Now,
        site: &mut crate::component::Site<'_>,
    ) -> Vec<Line<'static>> {
        placed(site.state)
            .iter()
            .filter(|toast| self.remaining(toast, now).is_some())
            .flat_map(|toast| {
                let props = bingo_sdk::component::Props::Toast(bingo_sdk::component::ToastProps {
                    text: toast.text.clone(),
                    until: toast.until,
                    alert: toast.alert,
                });
                site.draw(&toast.key, &props, |props, width| {
                    let bingo_sdk::component::Props::Toast(props) = props else {
                        return Vec::new();
                    };
                    vec![Line::from(Span::styled(
                        views::clip(&props.text, width),
                        theme::text(),
                    ))]
                })
            })
            .collect()
    }
    #[cfg(test)]
    pub fn lines(&self, state: &SessionState, now: Now, width: usize) -> Vec<Line<'static>> {
        self.components(
            now,
            &mut crate::component::Site {
                state,
                memo: &crate::pulled::Memo::default(),
                width,
                pulls: &mut Vec::new(),
            },
        )
    }
    pub fn next(&self, state: &SessionState, now: Now) -> Option<Instant> {
        placed(state)
            .iter()
            .filter_map(|toast| {
                self.remaining(toast, now)
                    .map(|remaining| now.instant + remaining)
            })
            .min()
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::*;
    use bingo_sdk::slot::Slot;
    #[test]
    fn toasts_disappear_at_their_deadline() {
        let (mut ui, now) = scene();
        let mut state = state();
        state.signals.entry("counter".into()).or_default().insert(
            "toast".into(),
            bingo_sdk::slot::with(
                serde_json::json!({}),
                [Slot::Toast {
                    text: "Saved counter".into(),
                    until: now.wall + std::time::Duration::from_secs(4),
                    alert: false,
                }],
            )
            .into(),
        );
        let tree = solo(&state);
        assert!(draw_tree(80, 24, &tree, &mut ui, now).contains("Saved counter"));
        assert!(!draw_tree(80, 24, &tree, &mut ui, later(now, 4000)).contains("Saved counter"));
    }
    #[test]
    fn a_holding_pane_never_replays_expired_toasts_and_alerts_only_once() {
        let (mut ui, now) = scene();
        let mut state = state();
        let mut slots = [
            Slot::Toast {
                text: "held".into(),
                until: now.wall + std::time::Duration::from_secs(4),
                alert: true,
            },
            Slot::Pane {
                id: "pane".into(),
                title: "Pane".into(),
                focus: false,
                close_on_escape: true,
                hold_toasts: true,
                asked: true,
                place: bingo_sdk::slot::above(),
                size: bingo_sdk::slot::PaneSize::default(),
                view: bingo_sdk::View::text("hold").into(),
            },
        ];
        slots.swap(0, 1);
        state.signals.entry("counter".into()).or_default().insert(
            "held".into(),
            bingo_sdk::slot::with(serde_json::json!({}), slots).into(),
        );
        assert!(!crate::pane::placed(&state).is_empty());
        assert!(ui.placed.toasts.sync(&state, now));
        assert!(!ui.placed.toasts.sync(&state, later(now, 6000)));
        assert!(ui.placed.toasts.next(&state, later(now, 6000)).is_none());
        assert_eq!(
            ui.placed.toasts.lines(&state, later(now, 6000), 80).len(),
            0
        );
        state
            .signals
            .get_mut("counter")
            .unwrap()
            .get_mut("held")
            .unwrap()["slots"]
            .as_array_mut()
            .unwrap()
            .retain(|slot| slot["kind"] != "pane");
        assert!(crate::pane::placed(&state).is_empty());
        ui.placed.toasts.sync(&state, later(now, 6000));
        assert_eq!(ui.placed.toasts.next(&state, later(now, 6000)), None);
        assert!(
            ui.placed
                .toasts
                .lines(&state, later(now, 10000), 80)
                .is_empty()
        );
    }
}
