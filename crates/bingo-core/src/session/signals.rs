//! Signal ownership changes are ordered by the same actor as their payloads.
use super::*;

pub(super) async fn publish(
    actor: &mut Actor,
    plugin: String,
    kind: String,
    payload: Value,
    generation: Option<ItemId>,
) -> Result<(), KernelError> {
    if payload.is_null()
        && generation.as_ref().is_some_and(|owner| {
            actor
                .state
                .signals
                .get(&plugin)
                .and_then(|kinds| kinds.get(&kind))
                .and_then(|signal| signal.generation())
                != Some(owner)
        })
    {
        return Ok(());
    }
    actor
        .publish(
            Event::Signal {
                plugin,
                kind,
                payload,
                generation,
            },
            None,
        )
        .await?;
    Ok(())
}

pub(super) async fn transfer(
    actor: &mut Actor,
    plugin: String,
    from: ItemId,
    to: ItemId,
    reply: tokio::sync::oneshot::Sender<Result<(), KernelError>>,
) {
    let result = handoff(actor, &plugin, &from, &to).await;
    let _ = reply.send(result);
}

async fn handoff(
    actor: &mut Actor,
    plugin: &str,
    from: &ItemId,
    to: &ItemId,
) -> Result<(), KernelError> {
    let signals: Vec<_> = actor
        .state
        .signals
        .get(plugin)
        .into_iter()
        .flat_map(|kinds| kinds.iter())
        .filter(|(_, signal)| signal.generation() == Some(from))
        .map(|(kind, signal)| (kind.clone(), signal.payload().clone()))
        .collect();
    for (kind, payload) in signals {
        publish(actor, plugin.into(), kind, payload, Some(to.clone())).await?;
    }
    Ok(())
}
