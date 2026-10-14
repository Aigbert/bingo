//! `/model` chooses for this session; `--keep` also writes the user layer.

use std::sync::{Arc, Weak};

use async_trait::async_trait;
use bingo_sdk::*;
use serde_json::json;

use crate::host::{Host, model_switch};
use crate::turn::ModelChoice;

const USAGE: &str = "usage: /model [<provider>/]<model>";

pub(super) struct ModelCommand {
    pub(super) host: Weak<Host>,
}

#[async_trait]
impl Command for ModelCommand {
    fn spec(&self) -> CommandSpec {
        super::spec(
            "model",
            "choose the model for this session",
            super::keep_args(ArgSpec::Catalog {
                source: "models".into(),
            }),
            true,
        )
    }

    async fn is_instant(&self, args: &str, state: &SessionState) -> bool {
        if args.trim().is_empty() || super::pin::parse(args).is_some() {
            return true;
        }
        let Ok(host) = super::host(&self.host) else {
            return true;
        };
        let Ok((wanted, _)) = super::choice_args(args) else {
            return true;
        };
        match target(&host, &state.summary.id, wanted).await {
            Ok(choice) => !crate::host::model_switch::needs_compaction(state, &choice),
            Err(_) => true,
        }
    }

    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let host = super::host(&self.host)?;
        if let Some((pin, id)) = super::pin::parse(args) {
            return super::pin::run(&host, cx, id, pin).await;
        }
        match args.trim() {
            "" => report(&host, cx).await,
            wanted => set(&host, cx, wanted).await,
        }
    }
}

/// Bare `/model`: what the session runs on — who serves it, which model, and
/// how hard it is asked to think — read from the resolved choice, which is
/// what the next turn will actually ask.
async fn report(host: &Arc<Host>, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
    let choice = running_on(host, cx).await?;
    Ok(CommandOutcome::View {
        view: View::text(format!("{}\n{USAGE}", said(&choice))),
    })
}

async fn target(
    host: &Host,
    session: &SessionId,
    wanted: &str,
) -> Result<ModelChoice, KernelError> {
    let known = host.catalog(CatalogKind::Providers).await?.entries;
    let (provider, model) = split(wanted, |id| known.iter().any(|p| p.id == id));
    model_switch::proposed_model(host, session, provider.map(str::to_owned), model.to_owned()).await
}

async fn set(
    host: &Arc<Host>,
    cx: &CommandContext,
    args: &str,
) -> Result<CommandOutcome, KernelError> {
    let (wanted, keep) = super::choice_args(args)?;
    let choice = target(host, &cx.session, wanted).await?;
    let pictures = !choice.capabilities.images
        && crate::host::model_switch::has_pictures(&host.session_state(&cx.session).await?);
    let compacted = model_switch::compact_for_switch(host, &cx.session, &choice).await?;
    if cx.cancel.is_cancelled() {
        return Err(KernelError::new(
            ErrorCode::Interrupted,
            "model switch cancelled",
        ));
    }
    host.reconfigure(
        &cx.session,
        SessionChange::Model {
            provider: Some(choice.endpoint.id().to_owned()),
            model: choice.id.clone(),
        },
    )
    .await?;
    let keys = [
        ("provider", json!(choice.endpoint.id())),
        ("model", json!(choice.id)),
    ];
    let mut message = format!(
        "⇄ {} · {} · thinking {} · {}",
        choice.id,
        choice.endpoint.id(),
        Effort::word(choice.reasoning),
        super::kept(host, keep, &keys)
    );
    if let Some((before, after)) = compacted {
        message.push_str(&format!(" · compacted {before} → {after}"));
    }
    if pictures {
        message.push_str(&format!(" · pictures stay, {} won't see them", choice.id));
    }
    Ok(CommandOutcome::Applied {
        message: Some(message),
    })
}

/// What the session's next turn runs on, resolved as the turn will resolve
/// it. A log session has none (ADR-0011 §1) and is told so.
async fn running_on(host: &Arc<Host>, cx: &CommandContext) -> Result<ModelChoice, KernelError> {
    host.session_model(&cx.session)
        .await?
        .ok_or_else(|| KernelError::new(ErrorCode::InvalidInput, "a log session has no model"))
}

/// The session's model and requested effort, in the vocabulary `/think` uses.
fn said(choice: &ModelChoice) -> String {
    let mut text = format!("model: {}/{}", choice.endpoint.id(), choice.id);
    if let Some(effort) = choice.reasoning {
        text.push_str(&format!("\nthinking: {}", effort.name()));
    }
    text
}

/// `provider/model` when the first segment names a provider; else the whole
/// text is the model and the provider stays.
fn split(text: &str, is_provider: impl Fn(&str) -> bool) -> (Option<&str>, &str) {
    match text.split_once('/') {
        Some((provider, model)) if is_provider(provider) && !model.is_empty() => {
            (Some(provider), model)
        }
        _ => (None, text),
    }
}

#[cfg(test)]
mod tests {
    use super::split;

    #[test]
    fn a_leading_provider_is_taken_only_when_it_is_one() {
        let known = |id: &str| id == "openai";
        assert_eq!(split("openai/gpt-5", known), (Some("openai"), "gpt-5"));
        assert_eq!(
            split("meta/llama-3", known),
            (None, "meta/llama-3"),
            "an unknown first segment is part of the model id"
        );
        assert_eq!(split("gpt-5", known), (None, "gpt-5"));
        assert_eq!(split("openai/", known), (None, "openai/"));
    }
}
