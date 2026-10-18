//! Latest wins; the fixed KeyBar row never queues or replays a toast.
use crate::{
    clock::Now,
    ui::{NOTICE, Ui},
};
use bingo_sdk::{SessionState, slot::Slot};
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};
#[derive(Debug, Default)]
pub struct State {
    seen: BTreeSet<String>,
    active: Option<(String, Instant)>,
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
            let (text, until, alert) = match placed.slot {
                Slot::Toast { text, until, alert } => (text, until, alert),
                _ => return None,
            };
            Some(Toast {
                key: format!(
                    "{}/{}/{}/{until}/{text}/{alert}",
                    state.summary.id, placed.plugin, placed.kind
                ),
                text,
                until,
                alert,
            })
        })
        .collect()
}
impl State {
    pub fn discard(&mut self) {
        self.active = None;
    }
    pub fn supersedes(&mut self, said: Instant) -> bool {
        if self.active.as_ref().is_some_and(|(_, at)| *at >= said) {
            true
        } else {
            self.discard();
            false
        }
    }

    pub fn sync(&mut self, state: &SessionState, now: Now) -> bool {
        let mut toasts = placed(state);
        toasts.sort_by_key(|toast| toast.until);
        // A key forgotten here is harmless if it comes back: its age is kept
        // below, so an expired toast is never said again.
        self.seen
            .retain(|key| toasts.iter().any(|toast| &toast.key == key));
        let mut alert = false;
        for toast in toasts {
            if self.seen.insert(toast.key.clone()) {
                // Preserve the publication's age: attaching late never starts a new life.
                let remaining = remaining(toast.until, now).min(NOTICE);
                let said = now
                    .instant
                    .checked_sub(NOTICE - remaining)
                    .unwrap_or(now.instant);
                self.active = Some((toast.key, said));
                alert |= toast.alert && !remaining.is_zero();
            }
        }
        if self
            .active
            .as_ref()
            .is_some_and(|(_, said)| now.instant >= *said + NOTICE)
        {
            self.active = None;
        }
        alert
    }
    fn active(&self, state: &SessionState, now: Now) -> Option<(Toast, Instant)> {
        let (key, said) = self.active.as_ref()?;
        let toast = placed(state).into_iter().find(|toast| &toast.key == key)?;
        (now.instant < *said + NOTICE && toast.until > now.wall).then_some((toast, *said))
    }
    pub fn next(&self, state: &SessionState, now: Now) -> Option<Instant> {
        let (toast, said) = self.active(state, now)?;
        Some((said + NOTICE).min(now.instant + remaining(toast.until, now)))
    }
    #[cfg(test)]
    pub fn lines(
        &self,
        state: &SessionState,
        now: Now,
        _: usize,
    ) -> Vec<ratatui::text::Line<'static>> {
        self.active(state, now)
            .map(|(toast, _)| vec![ratatui::text::Line::from(toast.text)])
            .unwrap_or_default()
    }
}
fn remaining(until: jiff::Timestamp, now: Now) -> Duration {
    if until > now.wall {
        until.duration_since(now.wall).unsigned_abs()
    } else {
        Duration::ZERO
    }
}
pub fn current(ui: &Ui, state: &SessionState, now: Now) -> Option<bingo_sdk::component::KeyToast> {
    let notice = ui.notice().filter(|notice| !notice.gone(now.instant));
    let placed = ui.placed.toasts.active(state, now);
    if let Some((toast, said)) = placed
        && notice.is_none_or(|notice| said > notice.said())
    {
        return Some(bingo_sdk::component::KeyToast {
            about: None,
            text: toast.text,
            level: bingo_sdk::Level::Info,
        });
    }
    notice.map(|notice| bingo_sdk::component::KeyToast {
        text: notice.text.clone(),
        level: notice.level,
        about: notice.about.clone(),
    })
}

pub fn components(
    ui: &Ui,
    state: &SessionState,
    now: Now,
    width: usize,
    pulls: &mut Vec<crate::pulled::Key>,
    next: &mut Option<std::time::Instant>,
) -> Option<bingo_sdk::component::KeyToast> {
    let mut toast = current(ui, state, now)?;
    let Some((placed, _)) = ui
        .placed
        .toasts
        .active(state, now)
        .filter(|(placed, _)| placed.text == toast.text)
    else {
        return Some(toast);
    };
    let props = bingo_sdk::component::Props::Toast(bingo_sdk::component::ToastProps {
        text: placed.text,
        until: placed.until,
        alert: placed.alert,
    });
    let key = crate::component::key(state, &placed.key, &props, width as u16, 1);
    let answer = key.as_ref().and_then(|key| ui.placed.pulled.get(key));
    if let Some(key) = key {
        pulls.push(key);
    }
    let drawing = crate::component::draw_marked(
        &props,
        answer,
        width,
        &crate::views::Marks::at(now),
        |props, _| {
            let bingo_sdk::component::Props::Toast(props) = props else {
                return Vec::new();
            };
            vec![ratatui::text::Line::from(props.text.clone())]
        },
    );
    *next = (*next).into_iter().chain(drawing.next).min();
    toast.text = drawing
        .lines
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" · ");
    Some(toast)
}

#[cfg(test)]
mod tests {
    use crate::test_support::*;
    use bingo_sdk::slot::Slot;
    #[test]
    fn a_new_publication_drops_the_local_toast_instead_of_replaying_it() {
        let (mut ui, now) = scene();
        ui.notify(bingo_sdk::Level::Info, "older", now.instant);
        let mut state = state();
        state.signals.entry("counter".into()).or_default().insert(
            "toast".into(),
            bingo_sdk::slot::with(
                serde_json::json!({}),
                [Slot::Toast {
                    text: "latest".into(),
                    until: later(now, 2900).wall,
                    alert: false,
                }],
            )
            .into(),
        );
        ui.sync_toasts(&state, later(now, 100));
        assert!(ui.notice().is_none());
        assert_eq!(
            super::current(&ui, &state, later(now, 100)).unwrap().text,
            "latest"
        );
        state.signals.clear();
        assert!(super::current(&ui, &state, later(now, 200)).is_none());
    }

    #[test]
    fn a_toast_that_comes_back_after_expiry_is_not_said_again() {
        let (_, now) = scene();
        let mut toasts = super::State::default();
        let mut state = state();
        let slot = bingo_sdk::slot::with(
            serde_json::json!({}),
            [Slot::Toast {
                text: "once".into(),
                until: later(now, 2000).wall,
                alert: true,
            }],
        );
        state
            .signals
            .entry("counter".into())
            .or_default()
            .insert("toast".into(), slot.clone().into());
        assert!(toasts.sync(&state, now), "a new alert rings");
        state.signals.clear();
        toasts.sync(&state, later(now, 100));
        assert!(
            toasts.seen.is_empty(),
            "a toast no longer placed is forgotten"
        );
        state
            .signals
            .entry("counter".into())
            .or_default()
            .insert("toast".into(), slot.into());
        let after = later(now, 2100);
        assert!(
            !toasts.sync(&state, after),
            "an expired toast rings no more"
        );
        assert!(toasts.next(&state, after).is_none());
        assert!(toasts.lines(&state, after, 80).is_empty());
    }

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
                hints: None,
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
