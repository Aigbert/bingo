//! Toast deadlines measured in visible time.
use crate::{clock::Now, theme, views};
use bingo_sdk::{SessionState, slot::Slot};
use ratatui::text::{Line, Span};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
#[derive(Debug, Default)]
pub struct State {
    clocks: BTreeMap<String, Clock>,
}
#[derive(Debug)]
struct Clock {
    paused: Duration,
    held: Option<Instant>,
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
        .enumerate()
        .filter_map(|(index, placed)| {
            if let Slot::Toast { text, until, alert } = placed.slot {
                Some(Toast {
                    key: format!(
                        "{}/{}/{}/{index}/{until}",
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
    pub fn sync(&mut self, state: &SessionState, holding: bool, now: Now) -> bool {
        let toasts = placed(state);
        self.clocks
            .retain(|key, _| toasts.iter().any(|toast| &toast.key == key));
        let mut alert = false;
        for toast in toasts {
            let unseen = !self.clocks.contains_key(&toast.key);
            alert |= unseen && toast.alert && toast.until > now.wall;
            let clock = self.clocks.entry(toast.key).or_insert(Clock {
                paused: Duration::ZERO,
                held: None,
            });
            match (clock.held, holding) {
                (Some(start), false) => {
                    clock.paused += now.since(start);
                    clock.held = None;
                }
                (None, true)
                    if toast
                        .until
                        .checked_add(clock.paused)
                        .is_ok_and(|until| until > now.wall) =>
                {
                    clock.held = Some(now.instant)
                }
                _ => {}
            }
        }
        alert
    }
    fn remaining(&self, toast: &Toast, now: Now) -> Option<Duration> {
        let paused = self.clocks.get(&toast.key).map_or(Duration::ZERO, |clock| {
            clock.paused + clock.held.map_or(Duration::ZERO, |start| now.since(start))
        });
        let until = toast.until.checked_add(paused).ok()?;
        (until > now.wall).then(|| until.duration_since(now.wall).unsigned_abs())
    }
    pub fn lines(&self, state: &SessionState, now: Now, width: usize) -> Vec<Line<'static>> {
        placed(state)
            .iter()
            .filter(|toast| self.remaining(toast, now).is_some())
            .map(|toast| Line::from(Span::styled(views::clip(&toast.text, width), theme::text())))
            .collect()
    }
    pub fn next(&self, state: &SessionState, now: Now) -> Option<Instant> {
        placed(state)
            .iter()
            .filter(|toast| {
                self.clocks
                    .get(&toast.key)
                    .is_none_or(|clock| clock.held.is_none())
            })
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
            ),
        );
        let tree = solo(&state);
        assert!(draw_tree(80, 24, &tree, &mut ui, now).contains("Saved counter"));
        assert!(!draw_tree(80, 24, &tree, &mut ui, later(now, 4000)).contains("Saved counter"));
    }
    #[test]
    fn a_holding_pane_pauses_visible_time_and_alerts_only_once() {
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
                rows: 8,
                view: bingo_sdk::View::text("hold").into(),
            },
        ];
        slots.swap(0,1);
        state.signals.entry("counter".into()).or_default().insert(
            "held".into(),
            bingo_sdk::slot::with(serde_json::json!({}), slots),
        );
        assert!(ui.panes.holding(&state));
        assert!(ui.toasts.sync(&state, ui.panes.holding(&state), now));
        assert!(!ui.toasts.sync(&state, true, later(now, 6000)));
        assert!(ui.toasts.next(&state, later(now, 6000)).is_none());
        assert_eq!(ui.toasts.lines(&state, later(now, 6000), 80).len(), 1);
        state.signals.get_mut("counter").unwrap().get_mut("held").unwrap()["slots"].as_array_mut().unwrap().retain(|slot|slot["kind"]!="pane");
        assert!(!ui.panes.holding(&state));
        ui.toasts.sync(&state, ui.panes.holding(&state), later(now, 6000));
        assert_eq!(
            ui.toasts.next(&state, later(now, 6000)),
            Some(now.instant + std::time::Duration::from_secs(10))
        );
        assert!(ui.toasts.lines(&state, later(now, 10000), 80).is_empty());
    }
}
