//! Process-local wakes delivered by owned, keyed alarms.
use crate::wake::{self, Wake};
use bingo_alarm::{Alarm, Alarms};
use bingo_sdk::{CancellationToken, Delivery, HostHandle, Input, IntentId, SessionId};
use jiff::Timestamp;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, Weak};

#[derive(Debug, Default)]
struct State {
    pending: HashMap<SessionId, Wake>,
    alarms: Alarms<SessionId>,
}

#[derive(Debug, Default)]
pub struct Wakes {
    state: Arc<Mutex<State>>,
    host: OnceLock<HostHandle>,
}

impl Wakes {
    pub fn set(&self, session: &SessionId, wake: Wake) -> Option<Wake> {
        let mut state = self.held();
        let previous = state.pending.insert(session.clone(), wake.clone());
        if let Some(host) = self.host.get() {
            state
                .alarms
                .arm(session.clone(), self.alarm(host, session, &wake));
        }
        previous
    }

    pub fn take(&self, session: &SessionId) -> Option<Wake> {
        let mut state = self.held();
        state.alarms.cancel(session);
        state.pending.remove(session)
    }

    /// Session-start hooks must return before their actor accepts writes.
    pub fn clear_stale(&self, host: HostHandle, session: SessionId) {
        let mut state = self.held();
        if state.pending.contains_key(&session) {
            return;
        }
        state.alarms.arm(
            session.clone(),
            Alarm::new(tokio::time::Instant::now(), async move {
                wake::publish(&host, &session, None).await;
            }),
        );
    }

    pub fn pending(&self, session: &SessionId) -> Option<Wake> {
        self.held().pending.get(session).cloned()
    }

    pub fn start(&self, host: HostHandle) {
        if self.host.set(host.clone()).is_err() {
            return;
        }
        let mut state = self.held();
        let wanted = state
            .pending
            .iter()
            .map(|(session, wake)| (session.clone(), self.alarm(&host, session, wake)))
            .collect::<Vec<_>>();
        state.alarms.reconcile(wanted);
    }

    pub fn stop(&self) {
        let mut state = self.held();
        state.alarms.clear();
        state.pending.clear();
    }

    fn alarm(&self, host: &HostHandle, session: &SessionId, wake: &Wake) -> Alarm {
        let state = Arc::downgrade(&self.state);
        let (host, session) = (host.clone(), session.clone());
        Alarm::new(
            bingo_alarm::deadline(wake.at, Timestamp::now()),
            async move {
                if let Some(wake) = spend(&state, &session) {
                    deliver(&host, &session, wake).await;
                }
            },
        )
    }

    fn held(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|held| held.into_inner())
    }

    #[cfg(test)]
    fn due(&self, now: Timestamp) -> Vec<(SessionId, Wake)> {
        let mut held = self.held();
        let mut due: Vec<_> = held
            .pending
            .iter()
            .filter(|(_, wake)| wake.at <= now)
            .map(|(session, wake)| (session.clone(), wake.clone()))
            .collect();
        for (session, _) in &due {
            held.pending.remove(session);
            held.alarms.cancel(session);
        }
        due.sort_by_key(|(_, wake)| wake.at);
        due
    }
    #[cfg(test)]
    fn next(&self) -> Option<Timestamp> {
        self.held().pending.values().map(|wake| wake.at).min()
    }
}

fn spend(state: &Weak<Mutex<State>>, session: &SessionId) -> Option<Wake> {
    state
        .upgrade()?
        .lock()
        .unwrap_or_else(|held| held.into_inner())
        .pending
        .remove(session)
}

pub async fn run(wakes: Arc<Wakes>, host: HostHandle, cancel: CancellationToken) {
    if !cancel.is_cancelled() {
        wakes.start(host);
    }
    cancel.cancelled().await;
    wakes.stop();
}

/// The note, on the conversation that asked for it: a delivery like any
/// other, so a session still in a turn takes it at the barrier and one that
/// is idle opens a turn on it. The pending wake is taken back first — it is
/// already forgotten here, and a screen that still counted one down would be
/// the only thing left saying so.
async fn deliver(host: &HostHandle, session: &SessionId, wake: Wake) {
    wake::publish(host, session, None).await;
    let input = Input::text(wake.note, wake::origin());
    if let Err(error) = host
        .deliver(session, IntentId::mint(), input, Delivery::Wake)
        .await
    {
        tracing::warn!(%session, "a wake reached no turn: {}", error.message);
    }
}

#[cfg(test)]
use crate::wake::WAKE_MOST;
#[cfg(test)]
use jiff::SignedDuration;
#[cfg(test)]
use std::time::Duration;
#[cfg(test)]
/// How long to sleep for a wake at `next`: never into the past, and never
/// longer than the longest wake there is, so a clock that jumped is not
/// slept through.
pub fn until(next: Option<Timestamp>, now: Timestamp) -> Duration {
    let ahead = next.map_or(WAKE_MOST, |at| at.duration_since(now));
    ahead.clamp(SignedDuration::ZERO, WAKE_MOST).unsigned_abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::Fixture;
    use serde_json::Value;

    fn session(name: &str) -> SessionId {
        SessionId::from_raw(name)
    }

    fn at(seconds: i64) -> Timestamp {
        Timestamp::UNIX_EPOCH + SignedDuration::from_secs(seconds)
    }

    fn wake(seconds: i64) -> Wake {
        Wake {
            at: at(seconds),
            note: format!("at {seconds}"),
        }
    }

    #[test]
    fn one_wake_stands_per_session_and_the_next_takes_its_place() {
        let wakes = Wakes::default();
        assert_eq!(wakes.set(&session("a"), wake(10)), None);
        assert_eq!(wakes.set(&session("a"), wake(20)), Some(wake(10)));
        assert_eq!(wakes.pending(&session("a")), Some(wake(20)));
        assert_eq!(wakes.pending(&session("b")), None);
        assert_eq!(wakes.take(&session("a")), Some(wake(20)));
        assert_eq!(wakes.take(&session("a")), None, "none stood");
    }

    #[test]
    fn what_is_due_is_taken_out_soonest_first_and_the_rest_is_waited_for() {
        let wakes = Wakes::default();
        wakes.set(&session("late"), wake(30));
        wakes.set(&session("soon"), wake(10));
        wakes.set(&session("later"), wake(50));
        assert_eq!(wakes.next(), Some(at(10)));
        assert_eq!(
            wakes.due(at(30)),
            vec![(session("soon"), wake(10)), (session("late"), wake(30))]
        );
        assert_eq!(wakes.next(), Some(at(50)), "the spent ones are gone");
        assert!(wakes.due(at(30)).is_empty(), "and are not due twice");
    }

    #[test]
    fn the_sleep_ends_when_the_wake_is_due_and_never_runs_past_the_longest_wake() {
        assert_eq!(until(Some(at(45)), at(10)), Duration::from_secs(35));
        assert_eq!(until(Some(at(0)), at(60)), Duration::ZERO);
        assert_eq!(until(None, at(0)), WAKE_MOST.unsigned_abs());
        assert_eq!(
            until(Some(at(86_400)), at(0)),
            WAKE_MOST.unsigned_abs(),
            "a clock that jumped is looked at again within the hour"
        );
    }

    /// A wake fires on the conversation it names and is spent by being
    /// forgotten: the delivery is `Wake` like any other, so a session still
    /// in a turn takes it at the barrier.
    #[tokio::test]
    async fn a_due_wake_lands_on_its_session_and_is_taken_back_from_the_screen_first() {
        let fixture = Fixture::new();
        let woken = session("ses_woken");
        let wakes = Wakes::default();
        wakes.set(&woken, wake(0));
        for (session, wake) in wakes.due(Timestamp::now()) {
            deliver(&fixture.handle(), &session, wake).await;
        }
        assert_eq!(wakes.pending(&woken), None, "spent");
        assert_eq!(
            fixture.host.delivered(),
            vec![(
                woken.clone(),
                Input::text("at 0", wake::origin()),
                Delivery::Wake
            )]
        );
        assert_eq!(
            fixture.host.extended(),
            vec![(
                woken,
                wake::PLUGIN.to_string(),
                wake::KIND.to_string(),
                Value::Null
            )],
            "the pending wake is taken back before the note lands"
        );
    }

    #[tokio::test]
    async fn a_cancelled_loop_does_not_start_a_due_delivery() {
        let fixture = Fixture::new();
        let wakes = Arc::new(Wakes::default());
        wakes.set(&session("ses_due"), wake(0));
        let cancel = CancellationToken::new();
        cancel.cancel();
        run(wakes, fixture.handle(), cancel).await;
        assert!(fixture.host.delivered().is_empty());
    }

    /// Setting an already-due wake rings the bell even while time is paused.
    #[tokio::test(start_paused = true)]
    async fn the_loop_delivers_a_wake_the_moment_it_is_set_for() {
        let fixture = Fixture::new();
        let wakes = Arc::new(Wakes::default());
        let cancel = CancellationToken::new();
        wakes.start(fixture.handle());
        wakes.set(&session("ses_now"), wake(0));
        tokio::time::timeout(Duration::from_secs(10), async {
            while fixture.host.delivered().is_empty() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the loop delivers the due wake");
        cancel.cancel();
        wakes.stop();
        assert_eq!(fixture.host.delivered().len(), 1, "delivered once");
    }
}
