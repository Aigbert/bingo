//! A switch acknowledges in the words the prototype says, each part in its own ink.
use bingo_sdk::*;
use serde_json::json;

/// The effort toast names the level in the strong ink and says what it means.
fn effort(effort: &str) -> Vec<view::TextChild> {
    vec![
        pick::bold(format!("effort {effort}"), "fg").into(),
        crate::draw::effort_words(effort).into(),
    ]
}

/// The model toast names the model, then its effort, then where it runs.
fn model(id: &str, effort: &str, provider: &str, context: Option<String>) -> Vec<view::TextChild> {
    let place = match context {
        Some(context) => format!("{provider}, {context} context"),
        None => provider.to_owned(),
    };
    vec![pick::bold(id, "fg").into(), effort.into(), place.into()]
}

pub async fn announce(cx: &HookContext, command: &str) -> Result<(), KernelError> {
    let attached = crate::bridge::attach(&cx.host, &cx.session).await?;
    let summary = &attached.snapshot.summary;
    let level = Effort::word(summary.effort);
    let parts = if command == "think" {
        effort(level)
    } else {
        let models = cx.host.catalog(CatalogKind::Models).await?.entries;
        let provider = summary.provider.as_deref().unwrap_or_default();
        let id = summary.model.as_deref().unwrap_or_default();
        let context = models
            .iter()
            .find(|m| m.id == format!("{provider}/{id}"))
            .and_then(|m| crate::card::known(&m.meta["context"]))
            .map(crate::card::tokens);
        model(id, level, provider, context)
    };
    let until = (std::time::SystemTime::now() + std::time::Duration::from_millis(2700))
        .try_into()
        .map_err(crate::error)?;
    cx.host
        .signal(
            &cx.session,
            "bingo.mod.models",
            "choice",
            slot::with(
                json!({}),
                [slot::Slot::Toast {
                    parts,
                    tone: Tone::Neutral,
                    until,
                    alert: false,
                }],
            ),
        )
        .await
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acknowledgements_keep_prototype_words_and_inks() {
        let parts = model("sonnet-5.5", "high", "anthropic", Some("200k".into()));
        assert_eq!(
            parts.iter().map(view::TextChild::fold).collect::<Vec<_>>(),
            ["sonnet-5.5", "high", "anthropic, 200k context"]
        );
        assert_eq!(parts[0], pick::bold("sonnet-5.5", "fg").into());
        let parts = effort("high");
        assert_eq!(parts[0], pick::bold("effort high", "fg").into());
        assert_eq!(parts[1], crate::draw::effort_words("high").into());
    }
}
