//! Side calls use the resolved provider and publish spend before returning.
use super::{Actor, Mailbox, Msg};
use crate::turn::ModelChoice;
use bingo_sdk::*;
use futures::StreamExt;
use tokio::sync::oneshot;

pub(super) type Reply = oneshot::Sender<Result<Vec<ModelEvent>, KernelError>>;

pub(super) fn complete(actor: &Actor, turn: Option<TurnId>, request: Completion, mut reply: Reply) {
    let selected = completion_model(actor, turn.as_ref());
    let (model, cancel) = match selected {
        Ok(value) => value,
        Err(error) => {
            let _ = reply.send(Err(error));
            return;
        }
    };
    let mailbox = actor.mailbox.clone();
    actor.tracker.spawn(async move {
        let _stop = cancel.clone().drop_guard();
        let result = tokio::select! {
            biased;
            _ = reply.closed() => return,
            _ = cancel.cancelled() => Err(KernelError::new(ErrorCode::Interrupted, "model call interrupted")),
            result = drain(&mailbox, turn, request, model, cancel.clone()) => result,
        };
        let _ = reply.send(result);
    });
}

fn completion_model(
    actor: &Actor,
    turn: Option<&TurnId>,
) -> Result<(ModelChoice, CancellationToken), KernelError> {
    let (config, cancel) = match turn {
        Some(turn) => {
            let running = actor
                .running
                .as_ref()
                .filter(|running| &running.turn == turn)
                .ok_or_else(|| {
                    KernelError::new(ErrorCode::NotReady, "the requested turn is not running")
                })?;
            (&running.config, running.cancel.child_token())
        }
        None => (&actor.config, actor.cancel.child_token()),
    };
    let model = config.model.clone().ok_or_else(|| {
        KernelError::new(ErrorCode::ProviderUnavailable, "the session has no model")
    })?;
    Ok((model, cancel))
}

pub(super) async fn charge(
    actor: &mut Actor,
    turn: Option<TurnId>,
    plugin: String,
    usage: Usage,
) -> Result<(), KernelError> {
    let turn = turn.filter(|turn| actor.is_running(turn));
    actor
        .publish(
            Event::UsageCharged {
                turn,
                plugin,
                usage,
            },
            None,
        )
        .await
        .map(|_| ())
}

async fn drain(
    mailbox: &Mailbox,
    turn: Option<TurnId>,
    mut completion: Completion,
    model: ModelChoice,
    cancel: CancellationToken,
) -> Result<Vec<ModelEvent>, KernelError> {
    completion.request.model = model.id;
    completion.request.max_tokens = completion.request.max_tokens.min(model.max_tokens);
    // A side question must not enter a stateful provider's main conversation.
    completion.request.session = None;
    let mut stream = model
        .provider
        .stream(completion.request, cancel)
        .await
        .map_err(provider_error)?;
    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        let event = event.map_err(provider_error)?;
        if let ModelEvent::Finish { usage, .. } = &event {
            mailbox
                .call(|reply| Msg::Charge {
                    turn: turn.clone(),
                    plugin: completion.plugin.clone(),
                    usage: *usage,
                    reply,
                })
                .await??;
        }
        events.push(event);
    }
    Ok(events)
}

fn provider_error(error: ProviderError) -> KernelError {
    KernelError::new(ErrorCode::ProviderUnavailable, error.to_string())
}
