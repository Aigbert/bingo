//! Signed delivery of one entry, shared by automatic and person-triggered fires.
use crate::{
    Entry, render,
    signed::{self, asked_origin},
};
use bingo_sdk::{
    Action, Attachment, CancellationToken, ClientIdentity, Delivery, ErrorCode, HostHandle, Input,
    IntentId, KernelError, Level, OpenOptions, Origin, SessionSelector, SessionSpec,
};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::Duration,
};
const REFUSED: &str = "schedule.refused";
const SURFACE: &str = crate::runner::SURFACE;
#[derive(Clone)]
pub(crate) struct Dispatch {
    pub host: HostHandle,
    pub trouble: Arc<Mutex<Option<String>>>,
    pub said: Arc<Mutex<HashSet<String>>>,
    pub cancel: CancellationToken,
}
impl Dispatch {
    pub async fn deliver(&self, entry: &Entry) -> Result<(), KernelError> {
        tokio::time::timeout(Duration::from_secs(30), self.turn(entry))
            .await
            .map_err(|_| KernelError::new(ErrorCode::Internal, "schedule delivery timed out"))?
    }
    /// The entry's own session, opened or continued, told what to do.
    async fn turn(&self, entry: &Entry) -> Result<(), KernelError> {
        let mut attachment = self.session(entry).await?;
        self.choose_mode(entry, &mut attachment).await;
        self.say(entry, &mut attachment).await
    }

    /// The session keyed `schedule/<id>`: the one this entry has been
    /// firing on all along, else a new one at the entry's directory.
    async fn session(&self, entry: &Entry) -> Result<Attachment, KernelError> {
        let selector = SessionSelector::ByKey { key: entry.key() };
        match self
            .host
            .open(selector, who(), OpenOptions::default())
            .await
        {
            Err(e) if e.code == ErrorCode::SessionNotFound => {
                let spec = SessionSelector::Create { spec: spec(entry) };
                self.host.open(spec, who(), OpenOptions::default()).await
            }
            other => other,
        }
    }

    /// The entry's mode, set before its text (ADR-0094 §2). The person's own
    /// entry asks for it as they would have typed it, so it may widen; any
    /// other entry's is the schedule's word and may only keep or tighten.
    /// A refusal leaves the session's mode and is said where a person looks.
    async fn choose_mode(&self, entry: &Entry, attachment: &mut Attachment) {
        let Some(mode) = &entry.permission_mode else {
            return;
        };
        let input = match entry.asked() {
            true => Input::text(format!("/permission {mode}"), asked_origin()),
            false => Input::action(
                Action {
                    name: "permission".into(),
                    args: serde_json::Value::String(mode.clone()),
                },
                origin(),
            ),
        };
        if let Err(error) = signed::submitted(attachment, input, &self.cancel).await {
            let why = format!(
                "its mode {mode} was refused ({}); the turn runs in the session's own mode",
                error.message
            );
            self.refused(entry, &why).await;
        }
    }

    /// The entry's text: the person's own command line runs as the command
    /// it names; everything else is prose to the model, delivered as a wake.
    async fn say(&self, entry: &Entry, attachment: &mut Attachment) -> Result<(), KernelError> {
        let command = bingo_sdk::command::command_line(&entry.text).is_some();
        if command && entry.asked() {
            let line = Input::text(entry.text.clone(), asked_origin());
            if let Err(error) = signed::submitted(attachment, line, &self.cancel).await {
                let why = format!("its command line was refused: {}", error.message);
                self.refused(entry, &why).await;
                return Err(error);
            }
            return Ok(());
        }
        if command {
            let why = format!(
                "no ask of the person's is on record for it, so its command line went to \
                 the model as prose; `/schedule confirm {}` makes it theirs",
                entry.id
            );
            self.refused(entry, &why).await;
        }
        let intent = IntentId::mint();
        self.host
            .deliver(
                &attachment.session,
                intent.clone(),
                Input::text(
                    entry.text.clone(),
                    if entry.asked() {
                        asked_origin()
                    } else {
                        origin()
                    },
                ),
                Delivery::Wake,
            )
            .await?;
        signed::acknowledged(attachment, &intent, &self.cancel).await
    }

    /// A refusal, where a person finds it: the `/schedule` badge every time,
    /// and a transcript notice the first time this process sees it.
    async fn refused(&self, entry: &Entry, why: &str) {
        let said = format!("schedule {}: {why}", entry.id);
        tracing::warn!("{said}");
        *self.trouble.lock().unwrap_or_else(|held| held.into_inner()) = Some(said.clone());
        let first = self
            .said
            .lock()
            .unwrap_or_else(|held| held.into_inner())
            .insert(said.clone());
        if first && let Err(error) = self.host.notice(Level::Warn, REFUSED, &said).await {
            tracing::debug!(%error, "a schedule refusal reached no transcript");
        }
    }
}

pub(crate) fn spec(entry: &Entry) -> SessionSpec {
    SessionSpec {
        cwd: entry.cwd.clone(),
        key: Some(entry.key()),
        title: Some(render::head(&entry.text, 40)),
        ..SessionSpec::default()
    }
}

fn who() -> ClientIdentity {
    ClientIdentity {
        name: SURFACE.into(),
        surface: SURFACE.into(),
    }
}

/// Who a scheduled turn is from. Nobody is at the keyboard, and the
/// transcript says so.
pub(crate) fn origin() -> Origin {
    Origin {
        render_surface: None,
        voice: Some(bingo_sdk::Voice::Quiet),
        surface: SURFACE.into(),
        principal: None,
        conversation: None,
    }
}

#[cfg(test)]
#[path = "delivery_tests.rs"]
mod tests;
