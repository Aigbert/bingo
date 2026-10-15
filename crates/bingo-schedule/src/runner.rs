//! The timer loop (ADR-0019 §3, §5): what fires, when, and on which
//! session.
//!
//! The loop itself owns no clock arithmetic and no policy. It asks
//! [`pass`] — pure — what is due and when to wake, opens or continues each
//! entry's own session, and delivers the text as a turn. Everything a fire
//! produces, including everything that goes wrong inside the turn, lands in
//! that session's transcript, which is the record `--resume` reads.
//!
//! Each entry has at most one in-flight delivery. Success moves its clock;
//! failure waits for a rescan before retrying, without holding up other entries.

use crate::{entry::Entry, store::Store};
use bingo_sdk::{CancellationToken, ErrorCode, KernelError};
use jiff::Zoned;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
pub const SURFACE: &str = "schedule";

/// How long the loop will sleep without looking at the store again. The
/// tools ring the bell when they change it; a person editing a file by hand
/// has nobody to ring it for them, and this is how long they wait.
const RESCAN: Duration = Duration::from_secs(60);

/// What one pass of the loop has to do.
#[derive(Debug, Default, PartialEq)]
pub struct Pass<'a> {
    /// Every entry whose next fire has come or gone: one fire each,
    /// however many occurrences were missed (ADR-0019 §5).
    pub due: Vec<&'a Entry>,
    /// The soonest fire still ahead, of the entries that are not due.
    pub next: Option<Zoned>,
}

/// What is due at `now`, and when to wake for the rest. Pure: the loop's
/// whole decision, testable without a clock or a host.
pub fn pass<'a>(entries: &'a [Entry], now: &Zoned) -> Pass<'a> {
    let mut pass = Pass::default();
    for entry in entries {
        let Some(fire) = entry.next_fire(now.time_zone()) else {
            continue;
        };
        if fire <= *now {
            pass.due.push(entry);
        } else if pass.next.as_ref().is_none_or(|soonest| fire < *soonest) {
            pass.next = Some(fire);
        }
    }
    pass
}

/// How long to sleep for a fire at `next`: never past the rescan, never
/// into the past.
pub fn delay(next: Option<&Zoned>, now: &Zoned) -> Duration {
    let Some(next) = next else { return RESCAN };
    let ahead = next.timestamp().duration_since(now.timestamp());
    match ahead.is_positive() {
        true => ahead.unsigned_abs().min(RESCAN),
        false => Duration::ZERO,
    }
}

/// The one runner this process has, if it took the store's claim.
pub struct Runner {
    store: Arc<Store>,
    changed: Arc<Notify>,
    dispatch: crate::delivery::Dispatch,
    cancel: CancellationToken,
}
impl Runner {
    pub(crate) fn new(
        store: Arc<Store>,
        changed: Arc<Notify>,
        dispatch: crate::delivery::Dispatch,
        cancel: CancellationToken,
    ) -> Self {
        Self {
            store,
            changed,
            dispatch,
            cancel,
        }
    }
    pub async fn run(self) {
        let runner = Arc::new(self);
        let mut flying = tokio::task::JoinSet::new();
        let mut held = HashSet::new();
        let mut retry = std::collections::HashMap::new();
        loop {
            let waited = runner.launch(&mut flying, &mut held, &mut retry);
            tokio::select! {
                biased;
                _ = runner.cancel.cancelled() => {
                    while flying.join_next().await.is_some() {}
                    return;
                }
                Some(result) = flying.join_next(), if !flying.is_empty() => {
                    if let Ok((id, ok)) = result {
                        held.remove(&id);
                        if !ok { retry.insert(id, tokio::time::Instant::now() + RESCAN); }
                    }
                }
                _ = runner.changed.notified() => {},
                _ = tokio::time::sleep(waited) => {},
            }
        }
    }
    fn launch(
        self: &Arc<Self>,
        flying: &mut tokio::task::JoinSet<(String, bool)>,
        held: &mut HashSet<String>,
        retry: &mut std::collections::HashMap<String, tokio::time::Instant>,
    ) -> Duration {
        if self.cancel.is_cancelled() {
            return Duration::ZERO;
        }
        let shelf = self.store.load();
        for bad in &shelf.unreadable {
            tracing::warn!(
                file = bad.file,
                "a schedule that cannot be read: {}",
                bad.why
            );
        }
        retry.retain(|_, until| *until > tokio::time::Instant::now());
        let now = Zoned::now();
        let pass = pass(&shelf.entries, &now);
        for entry in pass.due {
            if held.contains(&entry.id) || retry.contains_key(&entry.id) {
                continue;
            }
            held.insert(entry.id.clone());
            let runner = self.clone();
            let entry = entry.clone();
            flying.spawn(async move {
                let ok = runner.fire(&entry).await;
                (entry.id, ok)
            });
        }
        delay(pass.next.as_ref(), &now)
    }
    async fn fire(&self, entry: &Entry) -> bool {
        let result = match self.dispatch.deliver(entry).await {
            Ok(()) => spend(&self.store, entry),
            Err(error) => Err(error),
        };
        if let Err(error) = result {
            let said = format!(
                "{} opened no turn on {}: {}",
                entry.id,
                entry.key(),
                error.message
            );
            tracing::warn!("{said}");
            *self
                .dispatch
                .trouble
                .lock()
                .unwrap_or_else(|held| held.into_inner()) = Some(said);
            return false;
        }
        true
    }
}

pub(crate) fn spend(store: &Store, entry: &Entry) -> Result<(), KernelError> {
    // A concurrent pause, confirmation or deletion must not be overwritten by an old fire.
    if store.load().entries.iter().any(|current| current == entry) {
        store
            .save(&entry.fired(jiff::Timestamp::now()))
            .map_err(|error| KernelError::new(ErrorCode::Storage, error.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::tests::entry;
    use crate::tests::Fixture;
    use jiff::Timestamp;
    use jiff::tz::TimeZone;
    use std::sync::Mutex;

    /// The runner this process would have, over the fixture's own store.
    fn runner(fixture: &Fixture, trouble: &Arc<Mutex<Option<String>>>) -> Runner {
        let mut dispatch = fixture.schedules.dispatcher(fixture.handle());
        dispatch.trouble = trouble.clone();
        Runner::new(
            fixture.store(),
            Arc::new(Notify::new()),
            dispatch,
            CancellationToken::new(),
        )
    }

    fn at(seconds: i64) -> Zoned {
        Timestamp::from_second(seconds)
            .expect("a timestamp")
            .to_zoned(TimeZone::UTC)
    }

    /// Every entry is written at the epoch and fires every 30 minutes, so
    /// `at(n)` is n seconds after it was created.
    fn shelf(ids: &[&str]) -> Vec<Entry> {
        ids.iter()
            .map(|id| Entry {
                id: (*id).to_string(),
                ..entry()
            })
            .collect()
    }

    #[test]
    fn nothing_is_due_before_its_time_and_the_soonest_is_what_to_wake_for() {
        let entries = shelf(&["aaaa"]);
        let pass = pass(&entries, &at(60));
        assert!(pass.due.is_empty());
        assert_eq!(pass.next, Some(at(1800)));
        assert_eq!(
            delay(pass.next.as_ref(), &at(1795)),
            Duration::from_secs(5),
            "the sleep ends when the fire is due"
        );
    }

    #[test]
    fn an_entry_whose_hour_has_come_is_due_and_one_that_is_late_is_due_once() {
        let entries = shelf(&["aaaa"]);
        for now in [at(1800), at(1801), at(86_400)] {
            let pass = pass(&entries, &now);
            assert_eq!(pass.due.len(), 1, "at {now}");
            assert_eq!(pass.due[0].id, "aaaa");
            assert_eq!(pass.next, None, "a due entry is not also waited for");
        }
    }

    #[test]
    fn a_disabled_entry_is_neither_due_nor_waited_for() {
        let entries = vec![Entry {
            enabled: false,
            ..entry()
        }];
        assert_eq!(pass(&entries, &at(86_400)), Pass::default());
    }

    #[test]
    fn the_soonest_of_many_is_the_one_to_wake_for() {
        let mut entries = shelf(&["aaaa", "bbbb", "cccc"]);
        entries[0].spec = "every 2h".parse().expect("a spec");
        entries[1].spec = "every 45s".parse().expect("a spec");
        entries[2].spec = "every 10m".parse().expect("a spec");
        let pass = pass(&entries, &at(10));
        assert!(pass.due.is_empty());
        assert_eq!(pass.next, Some(at(45)), "the 45 second one");
        assert_eq!(delay(pass.next.as_ref(), &at(10)), Duration::from_secs(35));
    }

    #[test]
    fn a_fire_already_past_is_waited_for_no_time_at_all() {
        assert_eq!(delay(Some(&at(0)), &at(60)), Duration::ZERO);
    }

    #[test]
    fn a_store_with_nothing_ahead_of_it_still_looks_again() {
        assert_eq!(delay(None, &at(0)), RESCAN);
        assert_eq!(
            delay(Some(&at(86_400)), &at(0)),
            RESCAN,
            "a fire tomorrow is not a day asleep: a file may be edited by hand"
        );
    }

    /// A schedule is spent by having its fire written down, and opens a
    /// session of its own — which this host has none of, so the fire says so
    /// where a person looks.
    #[tokio::test]
    async fn a_schedule_of_a_persons_own_is_written_down_rather_than_forgotten() {
        let fixture = Fixture::new();
        fixture
            .schedules
            .store()
            .save(&entry())
            .expect("a schedule");

        let trouble = Arc::new(Mutex::new(None));
        runner(&fixture, &trouble).fire(&entry()).await;
        let shelf = fixture.shelf();
        assert_eq!(shelf.entries.len(), 1, "the file stays");
        assert!(
            shelf.entries[0].last_fired.is_none(),
            "failed delivery does not move its clock"
        );
        assert!(
            fixture.host.delivered().is_empty(),
            "a session of its own is opened first, and there is none"
        );
        let said = trouble
            .lock()
            .expect("the trouble")
            .clone()
            .unwrap_or_default();
        assert!(said.contains("schedule/abcd1234"), "{said}");
    }

    #[test]
    fn a_fire_opens_the_entry_s_own_session_at_the_entry_s_directory() {
        let spec = crate::delivery::spec(&entry());
        assert_eq!(spec.key.as_deref(), Some("schedule/abcd1234"));
        assert_eq!(spec.cwd, std::path::PathBuf::from("/work/project"));
        assert_eq!(
            spec.title.as_deref(),
            Some("check whether the nightly build is green"),
            "a session list says what the schedule does"
        );
        assert_eq!(spec.provider, None, "a schedule runs on the host's model");
        assert_eq!(crate::delivery::origin().surface, "schedule");
        assert_eq!(
            crate::delivery::origin().principal,
            None,
            "nobody is at the keyboard"
        );
    }
}

#[cfg(test)]
mod voice_tests {
    #[test]
    fn producer_stamps_its_voice() {
        assert_eq!(
            crate::delivery::origin().voice,
            Some(bingo_sdk::Voice::Quiet)
        );
    }
}
