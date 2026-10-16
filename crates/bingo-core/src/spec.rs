//! The spec and its durable fold. Changes are serialized per session; a
//! future change spanning parent and child would need a lock order.

use bingo_sdk::{SessionChange, SessionSpec, SessionState, SessionSummary};

use crate::turn::ModelChoice;

/// Apply a session setting to its owned spec.
pub(crate) fn apply(change: SessionChange, spec: &mut SessionSpec) {
    match change {
        SessionChange::Key(key) => spec.key = key,
        SessionChange::Model {
            provider,
            model,
            scope,
        } => {
            if provider.is_some() {
                spec.provider = provider;
            }
            spec.model = Some(model);
            spec.model_scope = Some(scope);
        }
        SessionChange::Thinking(level) => spec.thinking = Some(level),
        SessionChange::Title(title) => spec.title = crate::spec::titled(&title),
    }
}

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
        model_scope,
        system_extra,
        tools,
        thinking: _,
    } = spec;
    SessionSummary {
        choice: bingo_sdk::SessionModel {
            model: choice.map(|c| c.id.clone()),
            model_scope: choice.map(|_| {
                model_scope.unwrap_or(if spec.model.is_some() {
                    bingo_sdk::ChoiceScope::Session
                } else {
                    bingo_sdk::ChoiceScope::Default
                })
            }),
            effort: choice.and_then(|c| c.reasoning),
        },
        cwd: cwd.display().to_string(),
        key: key.clone(),
        parent: parent.clone(),
        title: title.clone(),
        driver: *driver,
        provider: choice.map(|c| c.endpoint.id().to_string()),
        system_extra: system_extra.clone(),
        tools: tools.clone(),
        ..became
    }
}

/// A blank title is no title: the session goes back to being named by
/// its first ask.
pub(crate) fn titled(title: &str) -> Option<String> {
    let title = title.trim();
    (!title.is_empty()).then(|| title.to_owned())
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
        model_scope: summary.model_scope,
        system_extra: summary.system_extra.clone(),
        tools: summary.tools.clone(),
        thinking: state
            .config
            .kernel
            .get("thinking")
            .and_then(|level| serde_json::from_value(level.clone()).ok()),
    }
}
