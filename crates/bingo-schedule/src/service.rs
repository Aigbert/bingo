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
            "{} {}",
            if enabled { "resumed" } else { "paused" },
            entry.id
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
    async fn run(&self, id: String) -> Result<String, KernelError> {
        let entry = self.entry(&id)?;
        if !entry.enabled {
            return Err(invalid("schedule is disabled; resume first"));
        }
        self.schedules
            .dispatcher(self.host()?.clone())
            .deliver(&entry)
            .await?;
        crate::runner::spend(self.schedules.store(), &entry)?;
        self.schedules.changed();
        Ok(format!(
            "ran {}; a concurrent runner fire may add one extra turn",
            entry.id
        ))
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
