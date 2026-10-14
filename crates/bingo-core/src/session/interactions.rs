//! Interactions: the actor opens one whenever something needs a person, holds
//! the reply channel until an answer arrives, and cancels the rest when the
//! turn that opened them ends.

use bingo_sdk::*;
use jiff::{SignedDuration, Timestamp};
use serde_json::Value;
use tokio::sync::oneshot;

use super::Actor;
use super::mailbox::Answered;

mod receipt;

/// A keyboard approval inside this window after opening is a stray keystroke.
pub const INTERACTION_GUARD_MS: i64 = 400;

pub(super) struct Pending {
    pub(super) interaction: Interaction,
    pub(super) reply: oneshot::Sender<Result<Answer, KernelError>>,
    /// Opened by a holding command, and closed when that command is done
    /// (ADR-0012 §5). The turn it names cannot say so: a question that
    /// arrived between turns names none either (ADR-0039 §1).
    by_command: bool,
}

impl Actor {
    pub(super) async fn receipt(&mut self, item: Item) -> Result<Item, KernelError> {
        let recorded = self
            .recorded_denial(&item.body)
            .and_then(|id| self.state.transcript.get(&id))
            .cloned();
        let canonical = if let Some(recorded) = recorded {
            recorded
        } else {
            self.publish(Event::ItemCompleted { item: item.clone() }, None)
                .await?;
            item
        };
        self.refresh_config().await?;
        Ok(canonical)
    }

    pub(super) fn recorded_denial(&self, body: &ItemBody) -> Option<ItemId> {
        let ItemBody::PermissionReceipt {
            item: Some(source),
            decision: DecisionKind::Deny,
            ..
        } = body
        else {
            return None;
        };
        self.state.transcript.items().iter().find_map(|previous| {
            matches!(&previous.body, ItemBody::PermissionReceipt { item: Some(id), decision: DecisionKind::Deny, .. } if id == source).then(|| previous.id.clone())
        })
    }

    pub(super) async fn open_interaction(
        &mut self,
        item: Option<ItemId>,
        kind: InteractionKind,
        answers: Vec<AnswerSpec>,
        reply: oneshot::Sender<Result<Answer, KernelError>>,
    ) -> Result<(), KernelError> {
        // A turn's call asks under the turn; a holding command (ADR-0012 §5)
        // and a question handed in between turns (ADR-0039 §1) ask under
        // none: an interaction is the session's, not the turn's.
        let turn = self.running.as_ref().map(|r| r.turn.clone());
        let by_command = self.commands.busy();
        let now = Timestamp::now();
        let interaction = Interaction {
            id: InteractionId::mint(),
            session: self.id.clone(),
            turn,
            item,
            opened_at: now,
            guard_until: now
                .checked_add(SignedDuration::from_millis(INTERACTION_GUARD_MS))
                .ok(),
            expires_at: None,
            kind,
            answers,
        };
        self.pending.insert(
            interaction.id.clone(),
            Pending {
                interaction: interaction.clone(),
                reply,
                by_command,
            },
        );
        self.publish(Event::InteractionOpened { interaction }, None)
            .await?;
        self.publish_summary(self.state.summary.clone()).await?;
        Ok(())
    }

    /// Why this answer cannot be taken, if it cannot.
    fn refuse(
        &self,
        id: &InteractionId,
        answer: &Answer,
        activation: Activation,
        plugin: bool,
    ) -> Option<(ErrorCode, String)> {
        let Some(pending) = self.pending.get(id) else {
            return Some((
                ErrorCode::InteractionClosed,
                "no such open interaction".into(),
            ));
        };
        if plugin && matches!(pending.interaction.kind, InteractionKind::Permission { .. }) {
            return Some((
                ErrorCode::PermissionDenied,
                "plugins cannot answer permission interactions".into(),
            ));
        }
        if !pending.interaction.answers.contains(&answer.spec()) {
            return Some((
                ErrorCode::InvalidInput,
                format!("{:?} is not an accepted answer here", answer.spec()),
            ));
        }
        if activation == Activation::Keyboard
            && !matches!(answer, Answer::Deny { .. } | Answer::Cancel)
            && let Some(guard) = pending.interaction.guard_until
            && Timestamp::now() < guard
        {
            return Some((
                ErrorCode::NotReady,
                "answered too soon after opening".into(),
            ));
        }
        None
    }

    pub(super) async fn answer(&mut self, answered: Answered) -> Result<(), KernelError> {
        let Answered {
            intent,
            interaction: id,
            answer,
            activation,
            who,
            plugin,
        } = answered;
        if let Some((code, message)) = self.refuse(&id, &answer, activation, plugin) {
            return self.reject(intent, code, message).await;
        }
        let Some(pending) = self.pending.remove(&id) else {
            return Ok(());
        };
        let receipt = receipt::answered(&pending.interaction, &answer);
        let _ = pending.reply.send(Ok(answer.clone()));
        self.publish(
            Event::InteractionResolved {
                id,
                answer,
                by: if plugin {
                    ResolvedBy::Plugin {
                        name: who.name,
                        surface: who.surface,
                    }
                } else {
                    ResolvedBy::Client {
                        name: who.name,
                        surface: who.surface,
                    }
                },
            },
            Some(intent.clone()),
        )
        .await?;
        if let Some(body) = receipt {
            self.record(body).await?;
        }
        self.applied(intent, Value::Null).await?;
        self.publish_summary(self.state.summary.clone()).await?;
        Ok(())
    }

    pub(super) async fn cancel_interactions(
        &mut self,
        reason: CancelReason,
    ) -> Result<(), KernelError> {
        let pending: Vec<_> = self.pending.drain().collect();
        self.cancel_each(pending, reason).await?;
        Ok(())
    }

    /// The holding command that asked is done: what it left open closes, and
    /// what anything else opened — a turn's, a door's — is not touched.
    pub(super) async fn cancel_command_interactions(&mut self) -> Result<(), KernelError> {
        let ids: Vec<InteractionId> = self
            .pending
            .iter()
            .filter(|(_, p)| p.by_command)
            .map(|(id, _)| id.clone())
            .collect();
        let pending: Vec<_> = ids
            .into_iter()
            .filter_map(|id| self.pending.remove(&id).map(|p| (id, p)))
            .collect();
        self.cancel_each(pending, CancelReason::CommandEnded)
            .await?;
        Ok(())
    }

    async fn cancel_each(
        &mut self,
        pending: Vec<(InteractionId, Pending)>,
        reason: CancelReason,
    ) -> Result<(), KernelError> {
        let changed = !pending.is_empty();
        for (id, pending) in pending {
            let _ = pending.reply.send(Err(KernelError::new(
                ErrorCode::InteractionClosed,
                format!("cancelled: {reason:?}"),
            )));
            self.publish(Event::InteractionCancelled { id, reason }, None)
                .await?;
        }
        if changed {
            self.publish_summary(self.state.summary.clone()).await?;
        }
        Ok(())
    }
}
