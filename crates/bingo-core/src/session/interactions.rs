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
    pub(super) reply: oneshot::Sender<Result<crate::gate::Answered, KernelError>>,
    /// Opened by a holding command, and closed when that command is done
    /// (ADR-0012 §5). The turn it names cannot say so: a question that
    /// arrived between turns names none either (ADR-0039 §1).
    by_command: bool,
}

impl Actor {
    pub(super) async fn receipt(
        &mut self,
        item: Item,
        reply: oneshot::Sender<Result<Item, KernelError>>,
    ) -> Result<(), KernelError> {
        let result = self.record_item(item).await;
        if let Err(error) = &result
            && error.code != ErrorCode::Interrupted
        {
            return Err(error.clone());
        }
        let _ = reply.send(result);
        Ok(())
    }

    pub(super) async fn record_item(&mut self, item: Item) -> Result<Item, KernelError> {
        let permission = matches!(item.body, ItemBody::PermissionReceipt { .. });
        let canonical = if let Some(recorded) = self.recorded_receipt(&item.body) {
            recorded
        } else {
            if permission
                && item
                    .turn
                    .as_ref()
                    .is_some_and(|turn| !self.is_running(turn))
            {
                return Err(KernelError::new(
                    ErrorCode::Interrupted,
                    "the receipt's turn has ended",
                ));
            }
            self.publish(Event::ItemCompleted { item: item.clone() }, None)
                .await?;
            item
        };
        if permission {
            self.refresh_config().await?;
        }
        Ok(canonical)
    }

    fn recorded_receipt(&self, body: &ItemBody) -> Option<Item> {
        let ItemBody::PermissionReceipt { interaction, .. } = body else {
            return None;
        };
        self.state.transcript.items().iter().find_map(|previous| {
            matches!(&previous.body, ItemBody::PermissionReceipt { interaction: id, .. }
                if id == interaction)
            .then(|| previous.as_ref().clone())
        })
    }

    pub(super) async fn open_interaction(
        &mut self,
        item: Option<ItemId>,
        kind: InteractionKind,
        answers: Vec<AnswerSpec>,
        reply: oneshot::Sender<Result<crate::gate::Answered, KernelError>>,
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
        let recorded = recorded_answer(&pending.interaction.kind, &answer);
        let receipt = self.answer_receipt(&pending.interaction, &recorded);
        let _ = pending.reply.send(Ok(crate::gate::Answered {
            ask: crate::gate::Asked {
                id: pending.interaction.id.clone(),
                opened_at: pending.interaction.opened_at,
            },
            answer,
        }));
        self.publish(
            Event::InteractionResolved {
                id,
                answer: recorded,
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
        if let Some(item) = receipt {
            self.record_item(item).await?;
        }
        self.applied(intent, Value::Null).await?;
        self.publish_summary(self.state.summary.clone()).await?;
        Ok(())
    }

    fn answer_receipt(&self, open: &Interaction, answer: &Answer) -> Option<Item> {
        let body = receipt::answered(open, answer)?;
        Some(self.fresh(open.turn.clone(), None, body))
    }

    pub(super) async fn cancel_interactions(
        &mut self,
        reason: CancelReason,
    ) -> Result<(), KernelError> {
        if let Some(running) = &mut self.running {
            let earliest = self
                .pending
                .values()
                .filter(|p| p.interaction.turn.as_ref() == Some(&running.turn))
                .map(|p| p.interaction.opened_at)
                .min();
            running.stopped_ask = running.stopped_ask.into_iter().chain(earliest).min();
        }
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

/// Login text is a credential, delivered only on the private reply channel.
fn recorded_answer(kind: &InteractionKind, answer: &Answer) -> Answer {
    match (kind, answer) {
        (InteractionKind::Login { .. }, Answer::Text { .. }) => Answer::Text {
            text: "[redacted]".into(),
        },
        _ => answer.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn login_text_is_redacted_but_cancellation_is_preserved() {
        let kind = InteractionKind::Login {
            provider: "local".into(),
            flow: LoginFlow::Paste,
        };
        let answer = Answer::Text {
            text: "a-secret".into(),
        };
        assert_eq!(
            recorded_answer(&kind, &answer),
            Answer::Text {
                text: "[redacted]".into()
            }
        );
        assert_eq!(recorded_answer(&kind, &Answer::Cancel), Answer::Cancel);
    }
}
