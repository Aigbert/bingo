//! The turn in flight: the task it runs on, the token that stops it, and the
//! config and tool set it is judged by for as long as it lasts. The actor
//! holds at most one, and everything the rest of the session may ask about
//! "the running turn" is asked of this.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;

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

pub(super) struct Running {
    pub(super) turn: TurnId,
    pub(super) cancel: CancellationToken,
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
        let cfg = Arc::clone(&actor.config);
        let mailbox = actor.mailbox.clone();
        let host = TurnMail {
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
            turn,
            cancel,
            task,
            config,
            tools: Vec::new(),
        }
    }

    /// Drop the turn where it stands: the loop is told to stop and the task is
    /// not waited for. Used only when the actor itself is going.
    pub(super) fn abandon(self) {
        self.cancel.cancel();
        self.task.abort();
    }
}

impl Actor {
    pub(super) async fn close_dropped_turn(
        &mut self,
        turn: &TurnId,
    ) -> Result<TurnOutcome, KernelError> {
        let open: Vec<_> = self.state.transcript.open_items(turn).cloned().collect();
        for mut item in open {
            item.status = ItemStatus::Interrupted;
            item.completed_at = Some(jiff::Timestamp::now());
            self.publish(Event::ItemCompleted { item }, None).await?;
        }
        let marker = self.fresh(
            Some(turn.clone()),
            None,
            ItemBody::Interruption {
                marker: INTERRUPTED_MARKER.into(),
            },
        );
        self.publish(Event::ItemCompleted { item: marker }, None)
            .await?;
        Ok(TurnOutcome {
            status: TurnStatus::Interrupted {
                reason: InterruptReason::UserCancel,
            },
            usage: self
                .state
                .turn
                .as_ref()
                .map(|live| live.usage)
                .unwrap_or_default(),
            items: self
                .state
                .transcript
                .items()
                .iter()
                .filter(|item| item.turn.as_ref() == Some(turn))
                .map(|item| item.as_ref().clone())
                .collect(),
        })
    }
}
