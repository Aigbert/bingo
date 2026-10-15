//! The timer loop (ADR-0019 §3, §5): what fires, when, and on which
//! session.
//!
//! The loop itself owns no clock arithmetic and no policy. It asks
//! [`pass`] — pure — what is due and when to wake, opens or continues each
//! entry's own session, and delivers the text as a turn. Everything a fire
//! produces, including everything that goes wrong inside the turn, lands in
//! that session's transcript, which is the record `--resume` reads.
//!
//! Each entry has at most one fire in flight, and one stalled fire holds up
//! no other. An occurrence is spent as soon as its session is open
//! ([`crate::delivery`]): however long the line runs, and whatever it
//! answers, that occurrence does not fire again. A session that could not be
//! opened leaves it unspent, and it waits a rescan before it is tried again,
//! as does a fire that panicked. A stamp the disk refused is remembered here,
//! so this process does not fire that occurrence twice either.

use crate::delivery::Fired;
use crate::{entry::Entry, store::Store};
use bingo_sdk::{CancellationToken, ErrorCode};
use jiff::{Timestamp, Zoned};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::task::{Id, JoinError, JoinSet};
use tokio::time::Instant;
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

/// What a fire task reports when it lands.
enum Landed {
    /// The occurrence is spent at this moment.
    Spent(Timestamp),
    /// Nothing was left to fire once the session opened.
    Skipped,
    /// No session opened: the occurrence stands, after a rescan.
    Unopened,
}

/// What the loop keeps between passes, beside the tasks themselves.
#[derive(Default)]
struct Flights {
    /// The entry each task in flight is firing.
    ids: HashMap<Id, String>,
    /// Entries whose last fire failed, and when they may be tried again.
    retry: HashMap<String, Instant>,
    /// The last fire spent per entry in this process, for a stamp the disk
    /// did not take.
    spent: HashMap<String, Timestamp>,
}

impl Flights {
    fn busy(&self, id: &str) -> bool {
        self.ids.values().any(|flying| flying == id) || self.retry.contains_key(id)
    }

    /// The entries as this process knows them: the disk, unless a fire spent
    /// here is newer than what the disk says.
    fn remembered(&mut self, entries: Vec<Entry>) -> Vec<Entry> {
        self.spent.retain(|id, at| {
            entries
                .iter()
                .any(|entry| &entry.id == id && entry.last_fired < Some(*at))
        });
        entries
            .into_iter()
            .map(|entry| match self.spent.get(&entry.id) {
                Some(at) => Entry {
                    last_fired: Some(*at),
                    enabled: entry.enabled && !entry.spec.is_once(),
                    ..entry
                },
                None => entry,
            })
            .collect()
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

    /// Fire what is due, sleep to the next one or a landing, and start again
    /// — until the plugin stops, when every fire in flight is let finish.
    pub async fn run(self) {
        let runner = Arc::new(self);
        let mut flying = JoinSet::new();
        let mut flights = Flights::default();
        loop {
            let waited = runner.launch(&mut flying, &mut flights);
            tokio::select! {
                biased;
                _ = runner.cancel.cancelled() => {
                    while flying.join_next().await.is_some() {}
                    return;
                }
                Some(joined) = flying.join_next_with_id(), if !flying.is_empty() => {
                    runner.landed(&mut flights, joined);
                }
                _ = runner.changed.notified() => {},
                _ = tokio::time::sleep(waited) => {},
            }
        }
    }

    /// Start a fire for every due entry not already in flight or waiting out
    /// a failure; how long until the next one is due.
    fn launch(self: &Arc<Self>, flying: &mut JoinSet<Landed>, flights: &mut Flights) -> Duration {
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
        flights.retry.retain(|_, until| *until > Instant::now());
        let entries = flights.remembered(shelf.entries);
        let now = Zoned::now();
        let pass = pass(&entries, &now);
        for entry in pass.due {
            if flights.busy(&entry.id) {
                continue;
            }
            let runner = self.clone();
            let fired = entry.clone();
            let task = flying.spawn(async move { runner.fire(&fired).await });
            flights.ids.insert(task.id(), entry.id.clone());
        }
        delay(pass.next.as_ref(), &now)
    }

    /// A fire task landed — or panicked, which frees its entry all the same.
    fn landed(&self, flights: &mut Flights, joined: Result<(Id, Landed), JoinError>) {
        let (task, landed) = match joined {
            Ok((task, landed)) => (task, Ok(landed)),
            Err(error) => (error.id(), Err(error)),
        };
        let Some(id) = flights.ids.remove(&task) else {
            return;
        };
        match landed {
            Ok(Landed::Spent(at)) => {
                flights.spent.insert(id, at);
            }
            Ok(Landed::Skipped) => {}
            Ok(Landed::Unopened) => {
                flights.retry.insert(id, Instant::now() + RESCAN);
            }
            Err(error) => {
                self.dispatch.troubled(format!(
                    "schedule {id}: its fire stopped unexpectedly: {error}"
                ));
                flights.retry.insert(id, Instant::now() + RESCAN);
            }
        }
    }

    async fn fire(&self, entry: &Entry) -> Landed {
        match self.dispatch.fire(&self.store, entry).await {
            Ok(Fired::Spent { at, .. }) => Landed::Spent(at),
            Ok(Fired::Skipped) => Landed::Skipped,
            Err(error) => {
                if error.code != ErrorCode::Interrupted {
                    self.dispatch.troubled(format!(
                        "{} opened no turn on {}: {}",
                        entry.id,
                        entry.key(),
                        error.message
                    ));
                }
                Landed::Unopened
            }
        }
    }
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
