//! Spending one occurrence: the entry's clock, written before its line is
//! submitted (ADR-0019 amendment 2026-10-16, ADR-0094 §5).
//!
//! A fire is spent once its session is open, never later: a line the
//! session took may run for an hour, and an answer that comes after any
//! deadline must not make the same occurrence fire again. The stamp lands on
//! the entry as it is now on disk, so a confirmation or edit made while the
//! session was opening is kept rather than overwritten by an older copy.

use bingo_sdk::{ErrorCode, KernelError};
use jiff::Timestamp;

use crate::{entry::Entry, store::Store};

/// One occurrence, spent.
#[derive(Debug)]
pub(crate) struct Spent {
    /// The entry as it stood on disk when it was spent: what fires.
    pub entry: Entry,
    /// The moment written as its last fire.
    pub at: Timestamp,
    /// Whether the stamp reached the disk. A failure is trouble to report,
    /// never a reason to fire the occurrence again.
    pub written: Result<(), KernelError>,
}

/// Spend `fired`'s occurrence at `at`: reread the entry by id and stamp the
/// current version. `None` when it is gone, paused, or already fired by
/// someone else since `fired` was read — nothing is spent and nothing fires.
pub(crate) fn spend(store: &Store, fired: &Entry, at: Timestamp) -> Option<Spent> {
    let current = current(store, fired)?;
    let written = store
        .save(&current.fired(at))
        .map(drop)
        .map_err(|error| KernelError::new(ErrorCode::Storage, error.to_string()));
    Some(Spent {
        entry: current,
        at,
        written,
    })
}

/// The entry `fired` names, as it is on disk now, if it still fires.
fn current(store: &Store, fired: &Entry) -> Option<Entry> {
    store
        .load()
        .entries
        .into_iter()
        .find(|entry| entry.id == fired.id)
        .filter(|entry| entry.enabled && entry.last_fired <= fired.last_fired)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::{Provenance, tests::entry};
    use crate::tests::Fixture;

    fn at() -> Timestamp {
        Timestamp::from_second(1_000_000).expect("a timestamp")
    }

    #[test]
    fn the_stamp_lands_on_the_entry_as_it_is_now() {
        let fixture = Fixture::new();
        let read = entry();
        let confirmed = Entry {
            by: Provenance::Person,
            ..read.clone()
        };
        fixture.store().save(&confirmed).expect("an entry");
        let spent = spend(&fixture.store(), &read, at()).expect("spent");
        assert_eq!(spent.entry, confirmed, "what fires is what stands now");
        assert!(spent.written.is_ok());
        assert_eq!(
            fixture.shelf().entries[0],
            confirmed.fired(at()),
            "the confirmation made mid-flight is kept, and the clock moved"
        );
    }

    #[test]
    fn a_gone_paused_or_already_fired_entry_is_not_spent() {
        let fixture = Fixture::new();
        let read = entry();
        assert!(spend(&fixture.store(), &read, at()).is_none(), "gone");
        let paused = Entry {
            enabled: false,
            ..read.clone()
        };
        fixture.store().save(&paused).expect("an entry");
        assert!(spend(&fixture.store(), &read, at()).is_none(), "paused");
        assert_eq!(fixture.shelf().entries[0], paused, "left as it was");
        let elsewhere = read.fired(Timestamp::UNIX_EPOCH + jiff::SignedDuration::from_secs(60));
        fixture.store().save(&elsewhere).expect("an entry");
        assert!(
            spend(&fixture.store(), &read, at()).is_none(),
            "another fire already spent it"
        );
    }

    #[test]
    fn a_stamp_that_cannot_be_written_still_spends_the_occurrence() {
        let fixture = Fixture::new();
        let read = entry();
        fixture.store().save(&read).expect("an entry");
        let blocker = fixture.dir().join(format!(".{}.tmp", read.id));
        std::fs::create_dir(&blocker).expect("a directory where the staging file goes");
        let spent = spend(&fixture.store(), &read, at()).expect("spent");
        assert_eq!(
            spent.written.expect_err("unwritten").code,
            ErrorCode::Storage
        );
        assert_eq!(spent.at, at());
    }
}
