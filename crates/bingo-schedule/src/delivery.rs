//! Signed delivery of one entry, shared by automatic and person-triggered fires.
//!
//! A fire has two halves. Opening the entry's session is the half that may
//! fail and be tried again: until a session is open, nothing ran. Once it is
//! open the occurrence is spent ([`crate::spend`]) and only then is the line
//! submitted. What the session answers is reported and nothing more: a
//! refusal sets the trouble badge and says so once (ADR-0094 §5), and no
//! answer within [`BOUND`] is a line still running — a long `!cargo test` or a
//! slow `/command` acknowledges only when it ends. Neither fires the
//! occurrence again, and neither can wait past the scheduler's stop.
use crate::{
    Entry, render,
    signed::{self, asked_origin},
    spend::{self, Spent},
    store::Store,
};
use bingo_sdk::{
    Action, Attachment, CancellationToken, ClientIdentity, Delivery, ErrorCode, HostHandle, Input,
    IntentId, KernelError, Level, OpenOptions, Origin, SessionSelector, SessionSpec,
};
use jiff::Timestamp;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::Duration,
};
const REFUSED: &str = "schedule.refused";
const SURFACE: &str = crate::runner::SURFACE;
/// How long a fire waits for its session to open, and then for each answer.
pub(crate) const BOUND: Duration = Duration::from_secs(30);

/// What the session said to a line it was handed.
#[derive(Debug, PartialEq)]
pub(crate) enum Answer {
    /// Applied, queued, or a turn started.
    Taken,
    /// Refused, or a command that ran and failed: the ack cannot tell them apart.
    Refused(KernelError),
    /// Nothing within [`BOUND`]: a command still running.
    Unanswered,
    /// The scheduler stopped while waiting.
    Stopped,
}

/// What became of a fire whose session opened.
#[derive(Debug, PartialEq)]
pub(crate) enum Fired {
    /// The occurrence is spent at `at`, whatever the line's answer.
    Spent { at: Timestamp, answer: Answer },
    /// Gone, paused or already fired elsewhere while opening: nothing ran.
    Skipped,
}

#[derive(Clone)]
pub(crate) struct Dispatch {
    pub host: HostHandle,
    pub trouble: Arc<Mutex<Option<String>>>,
    pub said: Arc<Mutex<HashSet<String>>>,
    pub cancel: CancellationToken,
}
impl Dispatch {
    /// One occurrence of `entry`. An error means no session opened: nothing
    /// ran and the occurrence stands for the caller to retry later.
    pub async fn fire(&self, store: &Store, entry: &Entry) -> Result<Fired, KernelError> {
        let mut attachment = self.opened(entry).await?;
        let Some(spent) = spend::spend(store, entry, Timestamp::now()) else {
            return Ok(Fired::Skipped);
        };
        self.unwritten(&spent);
        let answer = self.turn(&spent.entry, &mut attachment).await;
        Ok(Fired::Spent {
            at: spent.at,
            answer,
        })
    }

    /// The session, opened within [`BOUND`]. An open in progress is let
    /// finish, so a stop joins it; one that finishes after the stop is used
    /// for nothing, and the occurrence stands.
    async fn opened(&self, entry: &Entry) -> Result<Attachment, KernelError> {
        let attachment = tokio::time::timeout(BOUND, self.session(entry))
            .await
            .map_err(|_| {
                KernelError::new(ErrorCode::Internal, "its session did not open in time")
            })??;
        match self.cancel.is_cancelled() {
            true => Err(KernelError::new(
                ErrorCode::Interrupted,
                "the scheduler stopped",
            )),
            false => Ok(attachment),
        }
    }

    /// A stamp the disk refused is said; the occurrence is spent all the same.
    fn unwritten(&self, spent: &Spent) {
        if let Err(error) = &spent.written {
            self.troubled(format!(
                "schedule {}: fired, but its last fire was not written down ({}); \
                 this process will not fire that occurrence again",
                spent.entry.id, error.message
            ));
        }
    }

    /// Where a person finds what went wrong: the `/schedule` badge.
    pub(crate) fn troubled(&self, said: String) {
        tracing::warn!("{said}");
        *self.trouble.lock().unwrap_or_else(|held| held.into_inner()) = Some(said);
    }

    /// The mode, then the text, on the open session.
    async fn turn(&self, entry: &Entry, attachment: &mut Attachment) -> Answer {
        self.choose_mode(entry, attachment).await;
        self.say(entry, attachment).await
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
        let intent = signed::submit(attachment, input);
        let why = match answered(attachment, &intent, &self.cancel).await {
            Answer::Refused(error) => format!("refused ({})", error.message),
            Answer::Unanswered => format!("not answered within {}s", BOUND.as_secs()),
            Answer::Taken | Answer::Stopped => return,
        };
        let why = format!("its mode {mode} was {why}; the turn runs in the session's own mode");
        self.refused(entry, &why).await;
    }

    /// The entry's text: the person's own command line runs as the command
    /// it names; everything else is prose to the model, delivered as a wake.
    async fn say(&self, entry: &Entry, attachment: &mut Attachment) -> Answer {
        let command = bingo_sdk::command::command_line(&entry.text).is_some();
        let answer = match (command, entry.asked()) {
            (true, true) => {
                let line = Input::text(entry.text.clone(), asked_origin());
                let intent = signed::submit(attachment, line);
                answered(attachment, &intent, &self.cancel).await
            }
            (true, false) => {
                self.downgraded(entry).await;
                self.prose(entry, attachment).await
            }
            (false, _) => self.prose(entry, attachment).await,
        };
        if let Answer::Refused(error) = &answer {
            let why = format!("its text was refused or failed: {}", error.message);
            self.refused(entry, &why).await;
        }
        answer
    }

    /// A command line nobody asked for, said to be going to the model as prose.
    async fn downgraded(&self, entry: &Entry) {
        let why = format!(
            "no ask of the person's is on record for it, so its command line went to \
             the model as prose; `/schedule confirm {}` makes it theirs",
            entry.id
        );
        self.refused(entry, &why).await;
    }

    /// The text as a wake: the person's own entry signed as their line, any
    /// other as the schedule's.
    async fn prose(&self, entry: &Entry, attachment: &mut Attachment) -> Answer {
        let origin = match entry.asked() {
            true => asked_origin(),
            false => origin(),
        };
        let intent = IntentId::mint();
        let input = Input::text(entry.text.clone(), origin);
        let delivered = self
            .host
            .deliver(&attachment.session, intent.clone(), input, Delivery::Wake)
            .await;
        match delivered {
            Ok(()) => answered(attachment, &intent, &self.cancel).await,
            Err(error) => Answer::Refused(error),
        }
    }

    /// A refusal, where a person finds it: the `/schedule` badge every time,
    /// and a transcript notice the first time this process sees it.
    async fn refused(&self, entry: &Entry, why: &str) {
        let said = format!("schedule {}: {why}", entry.id);
        self.troubled(said.clone());
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

/// The answer to `intent`, waited for within [`BOUND`] and never past a stop.
async fn answered(
    attachment: &mut Attachment,
    intent: &IntentId,
    cancel: &CancellationToken,
) -> Answer {
    let waited = tokio::time::timeout(BOUND, signed::acknowledged(attachment, intent, cancel));
    match waited.await {
        Err(_) => Answer::Unanswered,
        Ok(Ok(())) => Answer::Taken,
        Ok(Err(error)) if error.code == ErrorCode::Interrupted => Answer::Stopped,
        Ok(Err(error)) => Answer::Refused(error),
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
