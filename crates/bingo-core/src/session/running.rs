//! The turn in flight: the task it runs on, the token that stops it, and the
//! config and tool set it is judged by for as long as it lasts. The actor
//! holds at most one, and everything the rest of the session may ask about
//! "the running turn" is asked of this.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bingo_sdk::*;
use futures::FutureExt;
use tokio::task::JoinHandle;

use super::mailbox::{Msg, TurnMail};
use super::{Actor, INTERRUPT_GRACE, TurnKind, panic_message};
use crate::turn::{INTERRUPTED_MARKER, TurnConfig, TurnOutcome, TurnRun, run_turn};

pub(crate) enum Ended {
    Returned(TurnOutcome),
    Panicked(String),
    Dropped,
}

/// Cancellation allows cooperative cleanup, then drops the entire future.
pub(super) async fn supervised<T>(
    future: impl std::future::Future<Output = T>,
    cancel: &CancellationToken,
) -> Option<T> {
    tokio::select! {
        biased;
        result = future => Some(result),
        _ = async {
            cancel.cancelled().await;
            tokio::time::sleep(INTERRUPT_GRACE).await;
        } => None,
    }
}

/// The door into the running turn (ADR-0036 §2): open while the model's
/// stream is and shut the moment it ends, because a turn deciding on what it
/// heard runs no call handed in from outside again. The turn sets it from its
/// own task; the actor reads it when a call knocks.
///
/// It also keeps how the turn ended for the calls it let in: one still running
/// when its turn completed on its own is dropped with the turn, and was not
/// interrupted by anybody.
#[derive(Clone, Default)]
pub(super) struct Door(Arc<DoorState>);

#[derive(Default)]
struct DoorState {
    open: AtomicBool,
    completed: AtomicBool,
}

impl Door {
    pub(super) fn set(&self, open: bool) {
        self.0.open.store(open, Ordering::Release);
    }

    pub(super) fn is_open(&self) -> bool {
        self.0.open.load(Ordering::Acquire)
    }

    /// Set before the turn's token is cancelled, so a call that sees the
    /// cancel already sees why.
    fn complete(&self) {
        self.0.completed.store(true, Ordering::Release);
    }

    /// Whether the turn ended on its own rather than by a person's `esc` or
    /// the session closing.
    pub(super) fn completed(&self) -> bool {
        self.0.completed.load(Ordering::Acquire)
    }
}

pub(super) struct Running {
    pub(super) stopped_ask: Option<jiff::Timestamp>,
    pub(super) hooks: Arc<tokio::sync::OnceCell<crate::turn::HookSet>>,
    pub(super) turn: TurnId,
    pub(super) cancel: CancellationToken,
    pub(super) stopped: Arc<std::sync::atomic::AtomicBool>,
    pub(super) door: Door,
    task: JoinHandle<()>,
    /// The config this turn started under; the session's may have been
    /// rebuilt since, and a call is judged by the turn's own.
    pub(super) config: Arc<TurnConfig>,
    /// The tools it resolved, once it has said so (`Msg::Offered`). Empty
    /// until then, which is the fail-closed reading: a call naming a tool
    /// nothing has offered yet is refused.
    pub(super) tools: Vec<Arc<dyn Tool>>,
}

impl Running {
    /// The turn loop runs in its own task and reports back by mail; a panic in
    /// it becomes a failed turn rather than a lost session.
    pub(super) fn spawn(
        actor: &Actor,
        turn: TurnId,
        cancel: CancellationToken,
        kind: TurnKind,
    ) -> Self {
        let run = TurnRun::new(turn.clone(), &actor.state.transcript, cancel.clone(), kind);
        let hooks = run.hooks.clone();
        let cfg = Arc::clone(&actor.config);
        let mailbox = actor.mailbox.clone();
        let stopped = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let door = Door::default();
        let host = TurnMail {
            stopped: stopped.clone(),
            door: door.clone(),
            mailbox: mailbox.clone(),
            turn: turn.clone(),
        };
        let config = Arc::clone(&cfg);
        let stop = cancel.clone();
        let task = tokio::spawn(async move {
            let future = AssertUnwindSafe(run_turn(&cfg, run, &host)).catch_unwind();
            let outcome = match supervised(future, &stop).await {
                Some(Ok(outcome)) => Ended::Returned(outcome),
                Some(Err(panic)) => Ended::Panicked(panic_message(panic)),
                None => Ended::Dropped,
            };
            mailbox.send(Msg::TurnFinished {
                turn: host.turn.clone(),
                outcome,
            });
        });
        Self {
            stopped_ask: None,
            hooks,
            stopped,
            door,
            turn,
            cancel,
            task,
            config,
            tools: Vec::new(),
        }
    }

    /// The turn is over and gone from the actor: whatever still runs under its
    /// token is dropped. A token nobody had cancelled is a turn that ended on
    /// its own, and the door says so to the calls it let in.
    pub(super) fn finish(self) {
        if !self.cancel.is_cancelled() {
            self.door.complete();
        }
        self.cancel.cancel();
    }

    /// Drop the turn where it stands: the loop is told to stop and the task is
    /// not waited for. Used only when the actor itself is going.
    pub(super) fn abandon(self) {
        self.cancel.cancel();
        self.task.abort();
    }
}

pub(super) async fn close_dropped_turn(
    actor: &mut Actor,
    turn: &TurnId,
) -> Result<TurnOutcome, KernelError> {
    let open: Vec<_> = actor.state.transcript.open_items(turn).cloned().collect();
    for mut item in open {
        item.status = ItemStatus::Interrupted;
        item.completed_at = Some(jiff::Timestamp::now());
        actor.publish(Event::ItemCompleted { item }, None).await?;
    }
    let asked_at = take_stopped_ask(actor, Some(turn));
    let marker = actor.fresh(
        Some(turn.clone()),
        None,
        ItemBody::Interruption {
            asked_at,
            marker: INTERRUPTED_MARKER.into(),
        },
    );
    actor
        .publish(Event::ItemCompleted { item: marker }, None)
        .await?;
    Ok(TurnOutcome {
        status: TurnStatus::Interrupted {
            reason: InterruptReason::UserCancel,
        },
        items: actor
            .state
            .transcript
            .items()
            .iter()
            .filter(|item| item.turn.as_ref() == Some(turn))
            .map(|item| item.as_ref().clone())
            .collect(),
    })
}

pub(super) fn take_stopped_ask(
    actor: &mut Actor,
    turn: Option<&TurnId>,
) -> Option<jiff::Timestamp> {
    actor
        .running
        .as_mut()
        .filter(|running| turn == Some(&running.turn))
        .and_then(|running| running.stopped_ask.take())
}
