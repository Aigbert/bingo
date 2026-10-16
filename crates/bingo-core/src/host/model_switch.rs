//! Resolve a proposed choice before mutating the session or its settings.

use super::*;
use crate::context::{ContextView, budget::Thresholds, estimate_tokens};
use futures::StreamExt;

pub(crate) async fn proposed_model(
    host: &Host,
    session: &SessionId,
    provider: Option<String>,
    model: String,
) -> Result<ModelChoice, KernelError> {
    let mut spec = host.live(session)?.spec;
    apply(
        SessionChange::Model {
            provider,
            model,
            scope: bingo_sdk::ChoiceScope::Session,
        },
        &mut spec,
    );
    host.model_for(&spec)
        .await?
        .ok_or_else(|| KernelError::new(ErrorCode::InvalidInput, "a log session has no model"))
}

pub(crate) async fn compact_for_switch(
    host: &Host,
    session: &SessionId,
    target: &ModelChoice,
) -> Result<Option<(u64, u64)>, KernelError> {
    let (state, mut events) = host.live(session)?.mailbox.attach().await?;
    if !needs_compaction(&state, target) {
        return Ok(None);
    }
    host.compact(session, None).await?;
    let before = context_used(&state);
    let mut compacted = false;
    let mut after = None;
    let mut reason = String::from("compaction produced no summary");
    while let Some(frame) = events.next().await {
        match frame.event {
            Event::Compacted { .. } => compacted = true,
            Event::TurnUsage { context, .. } if compacted => after = Some(context.used),
            Event::Notice { text, .. } => reason = text,
            Event::TurnCompleted { status, .. } => {
                return switch_compacted(
                    status,
                    after.map(|after| (before, after)),
                    target,
                    &reason,
                );
            }
            _ => {}
        }
    }
    Err(KernelError::new(
        ErrorCode::SessionClosed,
        "session closed while compacting",
    ))
}

pub(crate) fn needs_compaction(state: &SessionState, target: &ModelChoice) -> bool {
    compacts(
        state,
        context_used(state),
        target.endpoint.id(),
        &target.id,
        trigger(target),
    )
}

/// A switch compacts first when the session holds at least what the target
/// model's ruler allows; staying on the same model never does.
fn compacts(state: &SessionState, used: u64, provider: &str, model: &str, switch_at: u64) -> bool {
    let same = state.summary.provider.as_deref() == Some(provider)
        && state.summary.model.as_deref() == Some(model);
    !same && used >= switch_at
}

/// Publishes the switch decision for every model the catalogue gives a ruler.
/// The kernel's answer to `bingo.modelSwitch`: the decision `/model` acts on.
pub(super) fn service(key: &str) -> Option<Arc<dyn Any + Send + Sync>> {
    (key == bingo_sdk::switch::MODEL_SWITCH.name()).then(|| {
        let switches: Arc<dyn bingo_sdk::switch::ModelSwitch> = Arc::new(Switches);
        Arc::new(switches) as Arc<dyn Any + Send + Sync>
    })
}
struct Switches;
impl bingo_sdk::switch::ModelSwitch for Switches {
    fn catalog(&self, mut catalog: Catalog, state: &SessionState) -> Catalog {
        let used = context_used(state);
        for entry in &mut catalog.entries {
            let (Some((provider, model)), Some(switch_at)) =
                (entry.id.split_once('/'), entry.meta["switchAt"].as_u64())
            else {
                continue;
            };
            let decided = compacts(state, used, provider, model, switch_at);
            entry.meta["compacts"] = serde_json::json!(decided);
        }
        catalog
    }
}

fn context_used(state: &SessionState) -> u64 {
    let estimate = estimate_tokens(&[], &ContextView::model_view(&state.transcript), &[]);
    // An unfinished turn can have grown since the last measured response.
    state
        .context
        .map_or(estimate, |context| context.used.max(estimate))
}

fn trigger(target: &ModelChoice) -> u64 {
    Thresholds::of(target.capabilities.context_window, target.max_tokens).trigger
}

fn switch_compacted(
    status: TurnStatus,
    sizes: Option<(u64, u64)>,
    target: &ModelChoice,
    reason: &str,
) -> Result<Option<(u64, u64)>, KernelError> {
    if let TurnStatus::Failed { error } = status {
        return Err(error);
    }
    if matches!(status, TurnStatus::Interrupted { .. }) {
        return Err(KernelError::new(
            ErrorCode::Interrupted,
            "model switch interrupted",
        ));
    }
    if let Some((before, after)) = sizes
        && after < trigger(target)
    {
        return Ok(Some((before, after)));
    }
    let reason = sizes.map_or_else(
        || reason.to_owned(),
        |(_, after)| format!("compaction left {after} tokens"),
    );
    Err(KernelError::new(
        ErrorCode::InvalidInput,
        format!(
            "model unchanged: {reason}; context must be below {} tokens for {}",
            trigger(target),
            target.id
        ),
    ))
}

pub(crate) fn has_pictures(state: &SessionState) -> bool {
    ContextView::model_view(&state.transcript)
        .iter()
        .any(|message| pictures(&message.parts))
}

fn pictures(parts: &[ContentPart]) -> bool {
    parts.iter().any(|part| match part {
        ContentPart::Image(_) => true,
        ContentPart::ToolResult { parts, .. } => pictures(parts),
        _ => false,
    })
}
