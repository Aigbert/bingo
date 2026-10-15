//! `/think <level|off> [--keep]` chooses the requested reasoning effort.

use std::sync::{Arc, Weak};

use async_trait::async_trait;
use bingo_sdk::*;

use crate::host::Host;

pub(super) struct ThinkCommand {
    pub(super) host: Weak<Host>,
}

#[async_trait]
impl Command for ThinkCommand {
    fn now(&self, view: &command::SessionView<'_>) -> Option<String> {
        let state = view.state;
        let effort: Option<Effort> =
            serde_json::from_value(state.config.kernel["thinking"].clone()).unwrap_or_default();
        Some(Effort::word(effort).into())
    }

    fn spec(&self) -> CommandSpec {
        super::spec(
            "think",
            "set the reasoning effort",
            super::keep_args(ArgSpec::Words {
                person: vec![],
                alone: vec![],
                values: Effort::words().map(str::to_string).collect(),
                then: None,
            }),
            true,
        )
    }

    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let host = super::host(&self.host)?;
        match args.trim() {
            "" => report(&host, cx),
            wanted => set(&host, cx, wanted).await,
        }
    }
}

/// Bare `/think`: the level as it stands and how to say one.
fn report(host: &Host, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
    let level = host.session_thinking(&cx.session)?;
    Ok(CommandOutcome::View {
        view: View::text(format!("{}\nusage: /think {}", said(level), levels())),
    })
}

/// `/think <level|off>`: the next turn asks for this much.
async fn set(
    host: &Arc<Host>,
    cx: &CommandContext,
    wanted: &str,
) -> Result<CommandOutcome, KernelError> {
    let (wanted, keep) = super::choice_args(wanted)?;
    let next;
    let wanted = if wanted == "next" {
        let choice = host.session_model(&cx.session).await?.ok_or_else(|| {
            KernelError::new(ErrorCode::InvalidInput, "a log session has no model")
        })?;
        let id = format!("{}/{}", choice.endpoint.id(), choice.id);
        let entry = host
            .catalog(CatalogKind::Models)
            .await?
            .entries
            .into_iter()
            .find(|entry| entry.id == id);
        let efforts: Vec<String> = entry
            .and_then(|entry| serde_json::from_value(entry.meta["efforts"].clone()).ok())
            .unwrap_or_default();
        next = super::next::after(&efforts, Effort::word(host.session_thinking(&cx.session)?))
            .ok_or_else(|| {
                KernelError::new(
                    ErrorCode::InvalidInput,
                    "This model has no reasoning effort dial.",
                )
            })?
            .to_owned();
        &next
    } else {
        wanted
    };
    let level = Effort::spoken(wanted).ok_or_else(|| {
        KernelError::new(
            ErrorCode::InvalidInput,
            format!("unknown thinking level: {wanted}"),
        )
    })?;
    host.reconfigure(&cx.session, SessionChange::Thinking(level))
        .await?;
    let level = host.session_thinking(&cx.session)?;
    let message = format!(
        "{} · {}",
        said(level),
        super::kept(host, keep, &[("thinking", serde_json::json!(level))])
    );
    Ok(CommandOutcome::Applied {
        message: Some(message),
    })
}

fn said(level: Option<Effort>) -> String {
    format!("thinking: {}", Effort::word(level))
}

/// The ladder as a person says it, from the sdk's one list of words.
fn levels() -> String {
    format!("<{}>", Effort::words().collect::<Vec<_>>().join("|"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The usage line is the sdk's list and nothing kept beside it, so a
    /// level added there is offered here without a second edit.
    #[test]
    fn the_usage_line_lists_every_word_a_level_may_be_said_in() {
        assert_eq!(levels(), "<minimal|low|medium|high|xhigh|max|off>");
    }
}
