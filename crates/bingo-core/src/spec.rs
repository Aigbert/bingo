//! The spec and its durable fold. Changes are serialized per session; a
//! future change spanning parent and child would need a lock order.

use bingo_sdk::{SessionSpec, SessionState, SessionSummary};

use crate::turn::ModelChoice;

/// What the session was asked to be, over what it has become. The choice
/// alone says which provider and model actually answer.
pub(crate) fn stated(
    spec: &SessionSpec,
    choice: Option<&ModelChoice>,
    became: SessionSummary,
) -> SessionSummary {
    let SessionSpec {
        cwd,
        key,
        parent,
        title,
        driver,
        provider: _,
        model: _,
        system_extra,
        tools,
        thinking: _, // ConfigChanged carries the level, never the summary.
    } = spec;
    SessionSummary {
        cwd: cwd.display().to_string(),
        key: key.clone(),
        parent: parent.clone(),
        // A rename wins; an absent name never takes back one the actor minted.
        title: title.clone().or(became.title),
        driver: *driver,
        provider: choice.map(|c| c.endpoint.id().to_string()),
        model: choice.map(|c| c.id.clone()),
        system_extra: system_extra.clone(),
        tools: tools.clone(),
        ..became
    }
}

/// Last, not first: the journal's fold includes every change to the spec.
/// An absent level stays absent so resume can fill the settings default.
pub(crate) fn spec_of(state: &SessionState) -> SessionSpec {
    let summary = &state.summary;
    SessionSpec {
        cwd: summary.cwd.clone().into(),
        key: summary.key.clone(),
        parent: summary.parent.clone(),
        title: summary.title.clone(),
        driver: summary.driver,
        provider: summary.provider.clone(),
        model: summary.model.clone(),
        system_extra: summary.system_extra.clone(),
        tools: summary.tools.clone(),
        thinking: state
            .config
            .kernel
            .get("thinking")
            .and_then(|level| serde_json::from_value(level.clone()).ok()),
    }
}
