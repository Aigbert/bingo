//! Latest wins, except over a local notice that keeps its slot: a placed
//! toast arriving under one waits for it to end. Nothing is replayed.
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
    active: Option<Active>,
}
#[derive(Debug)]
struct Active {
    key: String,
    said: Instant,
    /// How long it waited for a notice that kept its slot; its placer's
    /// `until` moves by as much.
    waited: Duration,
}
struct Toast {
    key: String,
    parts: Vec<bingo_sdk::view::TextChild>,
    tone: bingo_sdk::Tone,
    until: jiff::Timestamp,
    alert: bool,
}
fn placed(state: &SessionState) -> Vec<Toast> {
    bingo_sdk::slot::placed(state)
        .into_iter()
        .filter_map(|placed| {
            let (parts, tone, until, alert) = match placed.slot {
                Slot::Toast {
                    parts,
                    tone,
                    until,
                    alert,
                } => (parts, tone, until, alert),
                _ => return None,
            };
            Some(Toast {
                key: format!(
                    "{}/{}/{}/{until}/{}/{tone:?}/{alert}",
                    state.summary.id,
                    placed.plugin,
                    placed.kind,
                    serde_json::to_string(&parts).unwrap_or_default()
                ),
                parts,
                tone,
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
        if self
            .active
            .as_ref()
            .is_some_and(|active| active.said >= said)
        {
            true
        } else {
            self.discard();
            false
        }
    }

    /// A placed toast already up gives the slot to a notice that keeps it,
    /// and takes it back at `end`.
    pub fn wait_for(&mut self, end: Instant) {
        if let Some(active) = &mut self.active {
            active.wait_for(end);
        }
    }

    /// `held` is when a local notice that keeps its slot ends.
    pub fn sync(&mut self, state: &SessionState, now: Now, held: Option<Instant>) -> bool {
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
                self.active = Some(Active {
                    key: toast.key,
                    said,
                    waited: Duration::ZERO,
                });
                alert |= toast.alert && !remaining.is_zero();
            }
        }
        if let Some(end) = held {
            self.wait_for(end);
        }
        if self
            .active
            .as_ref()
            .is_some_and(|active| now.instant >= active.said + NOTICE)
        {
            self.active = None;
        }
        alert
    }
    fn active(&self, state: &SessionState, now: Now) -> Option<(Toast, Instant)> {
        let active = self.active.as_ref()?;
        let toast = placed(state)
            .into_iter()
            .find(|toast| toast.key == active.key)?;
        let until = toast.until.checked_add(active.waited).ok()?;
        (active.said <= now.instant && now.instant < active.said + NOTICE && until > now.wall)
            .then_some((toast, active.said))
    }
}
impl Active {
    fn wait_for(&mut self, end: Instant) {
        if self.said < end {
            self.waited += end - self.said;
            self.said = end;
        }
    }
}
fn remaining(until: jiff::Timestamp, now: Now) -> Duration {
    if until > now.wall {
        until.duration_since(now.wall).unsigned_abs()
    } else {
        Duration::ZERO
    }
}
pub fn current(ui: &Ui, state: &SessionState, now: Now) -> Option<crate::ui::Notice> {
    placed_notice(ui, state, now).or_else(|| local(ui, now))
}

fn local(ui: &Ui, now: Now) -> Option<crate::ui::Notice> {
    ui.notice()
        .filter(|notice| !notice.gone(now.instant))
        .cloned()
}

/// The newest placed toast, while it is newer than the local one.
fn placed_notice(ui: &Ui, state: &SessionState, now: Now) -> Option<crate::ui::Notice> {
    let (toast, said) = ui.placed.toasts.active(state, now)?;
    local(ui, now)
        .is_none_or(|notice| said > notice.said())
        .then(|| crate::ui::Notice::placed(message(toast.parts, toast.tone), said))
}

fn message(parts: Vec<bingo_sdk::view::TextChild>, tone: bingo_sdk::Tone) -> crate::ui::NoticeKind {
    crate::ui::NoticeKind::Message(crate::ui::Message {
        about: None,
        parts,
        tone,
    })
}

/// A placed toast is a component a mod may draw; a local one is the surface's own.
pub fn components(
    ui: &Ui,
    state: &SessionState,
    now: Now,
    width: usize,
    pulls: &mut Vec<crate::pulled::Key>,
) -> Option<(crate::ui::Notice, Option<Vec<ratatui::text::Span<'static>>>)> {
    let Some(notice) = placed_notice(ui, state, now) else {
        return local(ui, now).map(|notice| (notice, None));
    };
    let (placed, _) = ui.placed.toasts.active(state, now)?;
    let props = bingo_sdk::component::Props::Toast(bingo_sdk::component::ToastProps {
        parts: placed.parts,
        tone: placed.tone,
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
        &crate::views::Marks::default(),
        |props, _| {
            let bingo_sdk::component::Props::Toast(props) = props else {
                return Vec::new();
            };
            vec![ratatui::text::Line::from(crate::tone::parts(
                &props.parts,
                props.tone,
                crate::tone::Content::Plain,
                None,
            ))]
        },
    );
    let spans = drawing.lines.into_iter().flat_map(|line| line.spans);
    Some((notice, Some(spans.collect())))
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
                    tone: bingo_sdk::Tone::Neutral,
                    parts: vec!["latest".into()],
                    until: later(now, 2900).wall,
                    alert: false,
                }],
            )
            .into(),
        );
        ui.sync_toasts(&state, later(now, 100));
        assert!(ui.notice().is_none());
        assert_eq!(
            super::current(&ui, &state, later(now, 100))
                .unwrap()
                .plain()
                .unwrap()
                .text,
            "latest"
        );
        state.signals.clear();
        assert!(super::current(&ui, &state, later(now, 200)).is_none());
    }

    /// The resume toast keeps its slot whichever lands first, and the key
    /// hint takes the slot after it. No capture holds two toasts.
    #[test]
    fn the_resume_toast_wins_and_the_hint_waits_for_its_slot() {
        let hint = |now: crate::clock::Now| {
            let mut state = state();
            state.signals.entry("palette".into()).or_default().insert(
                "key-hint".into(),
                bingo_sdk::slot::with(
                    serde_json::json!({}),
                    [Slot::Toast {
                        tone: bingo_sdk::Tone::Neutral,
                        parts: vec!["hint".into()],
                        until: later(now, 2800).wall,
                        alert: false,
                    }],
                )
                .into(),
            );
            state
        };

        fn shown(
            ui: &crate::ui::Ui,
            state: &bingo_sdk::SessionState,
            at: crate::clock::Now,
        ) -> Option<String> {
            super::current(ui, state, at).and_then(|notice| notice.plain().map(|p| p.text))
        }
        for hint_first in [true, false] {
            let (mut ui, now) = scene();
            let state = hint(now);
            let resumed = bingo_sdk::toast::Say::ok("resumed · fix the build".to_string());
            if hint_first {
                ui.sync_toasts(&state, now);
                ui.arrive(resumed, now.instant);
                ui.draws_notice(now.instant);
            } else {
                ui.arrive(resumed, now.instant);
                ui.draws_notice(now.instant);
                ui.sync_toasts(&state, later(now, 100));
            }
            let ends = later(now, 2700);
            ui.sync_toasts(&state, later(now, 1000));
            assert_eq!(
                shown(&ui, &state, later(now, 1000)).as_deref(),
                Some("resumed · fix the build")
            );
            ui.sync_toasts(&state, ends);
            assert_eq!(shown(&ui, &state, ends).as_deref(), Some("hint"));
        }
    }

    #[test]
    fn a_toast_that_comes_back_after_expiry_is_not_said_again() {
        let (_, now) = scene();
        let mut toasts = super::State::default();
        let mut state = state();
        let slot = bingo_sdk::slot::with(
            serde_json::json!({}),
            [Slot::Toast {
                tone: bingo_sdk::Tone::Neutral,
                parts: vec!["once".into()],
                until: later(now, 2000).wall,
                alert: true,
            }],
        );
        state
            .signals
            .entry("counter".into())
            .or_default()
            .insert("toast".into(), slot.clone().into());
        assert!(toasts.sync(&state, now, None), "a new alert rings");
        state.signals.clear();
        toasts.sync(&state, later(now, 100), None);
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
            !toasts.sync(&state, after, None),
            "an expired toast rings no more"
        );
        assert!(toasts.active(&state, after).is_none());
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
                    tone: bingo_sdk::Tone::Neutral,
                    parts: vec!["Saved counter".into()],
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
                tone: bingo_sdk::Tone::Neutral,
                parts: vec!["held".into()],
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
        assert!(ui.placed.toasts.sync(&state, now, None));
        assert!(!ui.placed.toasts.sync(&state, later(now, 6000), None));
        assert!(ui.placed.toasts.active(&state, later(now, 6000)).is_none());
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
        ui.placed.toasts.sync(&state, later(now, 6000), None);
        assert!(ui.placed.toasts.active(&state, later(now, 6000)).is_none());
    }
}
