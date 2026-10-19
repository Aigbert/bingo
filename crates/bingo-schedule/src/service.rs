use crate::delivery::{Answer, BOUND, Fired};
use crate::{Schedules, render, tools};
use bingo_sdk::*;
use bingo_services::schedule::{self, Listing, Pending, Service};
use std::sync::{Arc, OnceLock};

#[derive(Debug)]
pub struct Owner {
    pub schedules: Arc<Schedules>,
    pub host: OnceLock<HostHandle>,
}
fn storage(error: std::io::Error) -> KernelError {
    KernelError::new(ErrorCode::Storage, error.to_string())
}
pub fn invalid(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::InvalidInput, error.to_string())
}
impl Owner {
    pub fn new(schedules: Arc<Schedules>, host: HostHandle) -> Self {
        Self {
            schedules,
            host: OnceLock::from(host),
        }
    }
    fn host(&self) -> Result<&HostHandle, KernelError> {
        self.host
            .get()
            .ok_or_else(|| invalid("schedule service has not started"))
    }
    fn entry(&self, id: &str) -> Result<crate::Entry, KernelError> {
        crate::command::consent::persons_own("change")?;
        if id.trim().is_empty() {
            return Err(invalid("expected a schedule id"));
        }
        tools::find(&self.schedules.store().load(), id)
            .cloned()
            .map_err(invalid)
    }
    fn enabled(&self, id: &str, enabled: bool) -> Result<String, KernelError> {
        let mut entry = self.entry(id)?;
        entry.enabled = enabled;
        self.schedules.store().save(&entry).map_err(storage)?;
        self.schedules.changed();
        Ok(format!(
            "{} · {}",
            if enabled { "resumed" } else { "paused" },
            entry.text
        ))
    }
}
#[async_trait::async_trait]
impl Service for Owner {
    async fn list(&self) -> Result<Listing, KernelError> {
        let sessions = self.host()?.sessions(SessionFilter::default()).await?;
        let tz = jiff::tz::TimeZone::system();
        let entries = self
            .schedules
            .store()
            .load()
            .entries
            .iter()
            .map(|entry| {
                let session = sessions
                    .iter()
                    .find(|s| s.key.as_deref() == Some(entry.key().as_str()));
                render::row_of(entry, &tz, session.map(|s| s.id.clone()))
            })
            .collect();
        Ok(Listing {
            entries,
            holder: self.schedules.holder(),
            trouble: self.schedules.trouble(),
        })
    }
    async fn pending(&self, session: SessionId) -> Result<Option<Pending>, KernelError> {
        Ok(self
            .schedules
            .wakes()
            .pending(&session)
            .map(|wake| Pending {
                at: wake.at.to_string(),
                note: wake.note,
            }))
    }
    async fn pause(&self, id: String) -> Result<String, KernelError> {
        self.enabled(&id, false)
    }
    async fn resume(&self, id: String) -> Result<String, KernelError> {
        self.enabled(&id, true)
    }
    async fn forget(&self, id: String) -> Result<String, KernelError> {
        let entry = self.entry(&id)?;
        self.schedules.store().delete(&entry.id).map_err(storage)?;
        self.schedules.changed();
        Ok(format!("forgot {}; its session stays", entry.id))
    }
    /// Run one occurrence now. Typed in the entry's own session this cannot
    /// wait on itself: `/schedule` is an instant command run in a task of its
    /// own, and the line it hands the session is answered on that session's
    /// stream — within the bound, or reported as still running.
    async fn run(&self, id: String) -> Result<String, KernelError> {
        let entry = self.entry(&id)?;
        if !entry.enabled {
            return Err(invalid("schedule is disabled; resume first"));
        }
        let fired = self
            .schedules
            .dispatcher(self.host()?.clone())
            .fire(self.schedules.store(), &entry)
            .await?;
        self.schedules.changed();
        ran(&entry.id, fired)
    }
}

/// What the person is told of a run they asked for. The occurrence is spent
/// in every case but `Skipped`; a refusal is also on the `/schedule` badge.
fn ran(id: &str, fired: Fired) -> Result<String, KernelError> {
    let answer = match fired {
        Fired::Skipped => {
            return Err(invalid(format!(
                "schedule {id} changed while its session opened; nothing ran"
            )));
        }
        Fired::Spent { answer, .. } => answer,
    };
    match answer {
        Answer::Taken => Ok(format!(
            "ran {id}; a fire in another process may add one extra turn"
        )),
        Answer::Unanswered => Ok(format!(
            "started {id}; it is still running after {}s",
            BOUND.as_secs()
        )),
        Answer::Refused(error) => Err(KernelError::new(
            error.code,
            format!(
                "ran {id}, and its line was refused or failed: {}",
                error.message
            ),
        )),
        Answer::Stopped => Err(KernelError::new(
            ErrorCode::Interrupted,
            format!("started {id}; the scheduler stopped before it answered"),
        )),
    }
}
pub fn contribution(owner: Arc<Owner>) -> Contribution {
    let service: Arc<dyn Service> = owner;
    Contribution::service(
        &schedule::KEY,
        service.clone(),
        Some(schedule::wire(service)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_taken_run_names_only_the_cross_process_race() {
        let message = ran(
            "abcd",
            Fired::Spent {
                at: jiff::Timestamp::UNIX_EPOCH,
                answer: Answer::Taken,
            },
        )
        .unwrap();
        assert_eq!(
            message,
            "ran abcd; a fire in another process may add one extra turn"
        );
    }
}
