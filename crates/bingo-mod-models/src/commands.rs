use bingo_sdk::*;
use serde_json::{Value, json};
pub struct Commands(pub crate::Shared);
#[async_trait::async_trait]
impl Interceptor for Commands {
    fn id(&self) -> &str {
        "models.commands"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher::at(catalogue::COMMAND_RUN)
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        cx: &HookContext,
        value: Value,
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let command: catalogue::CommandRun =
            serde_json::from_value(value.clone()).map_err(crate::error)?;
        if command.surface != Some(render::SurfaceName::Terminal) {
            return next.run(value).await;
        }
        let name = value["command"].as_str().unwrap_or_default();
        let args = value["args"].as_str().unwrap_or_default().trim();
        if !opens(name, args, &cx.host).await? {
            return outcome(next.run(value).await?);
        }
        let fallback = fallback(args, &value, next).await;
        let (picker, providers) = opening(name, args, cx).await?;
        self.0
            .lock()
            .map_err(crate::error)?
            .insert(cx.session.clone(), picker.clone());
        crate::state::publish(&cx.host, &cx.session, &picker).await?;
        probe(&cx.host, &cx.session, &providers);
        Ok(json!({"kind":"surface","action":{"name":"ModelPicker","args":{}},"fallback":fallback}))
    }
}
/// Ask every provider not yet heard from, all at once and after the picker is up: its
/// rows answer as each probe does, and the command never waits on an endpoint.
fn probe(host: &HostHandle, session: &SessionId, providers: &[CatalogEntry]) {
    let refreshes: Vec<String> = providers
        .iter()
        .filter(|p| crate::label::reach(p) == crate::label::Reach::Unchecked)
        .map(|p| format!("refresh {}", p.id))
        .collect();
    if refreshes.is_empty() {
        return;
    }
    let (host, session) = (host.clone(), session.clone());
    tokio::spawn(async move {
        futures::future::join_all(
            refreshes
                .iter()
                .map(|args| crate::bridge::run(&host, &session, "models", args, None)),
        )
        .await;
    });
}
async fn opens(name: &str, args: &str, host: &HostHandle) -> Result<bool, KernelError> {
    match name {
        "think" | "models" => Ok(args.is_empty()),
        "login" | "logout" => Ok(args.is_empty()),
        "model" if args.is_empty() => Ok(true),
        "model" if filters(args) => {
            let models = host.catalog(CatalogKind::Models).await?.entries;
            Ok(!models
                .iter()
                .any(|m| m.id == args || m.id.split_once('/').is_some_and(|(_, id)| id == args)))
        }
        _ => Ok(false),
    }
}
/// A `provider/model` argument names a model outright, listed or not, so it is the kernel's to switch.
fn filters(args: &str) -> bool {
    !args.contains(char::is_whitespace) && !args.contains('/') && args != "next"
}

async fn fallback(args: &str, value: &Value, next: Next<'_, Value, Value>) -> String {
    if args.is_empty() {
        next.run(value.clone())
            .await
            .ok()
            .and_then(|value| serde_json::from_value::<catalogue::CommandReply>(value).ok())
            .map(CommandOutcome::from)
            .and_then(|outcome| match outcome {
                CommandOutcome::View { view } => Some(view.fold()),
                CommandOutcome::Applied { message } => message,
                _ => None,
            })
    } else {
        None
    }
    .unwrap_or_else(|| "Choose with /model <provider/model>; /model next cycles pins.".into())
}
async fn opening(
    name: &str,
    args: &str,
    cx: &HookContext,
) -> Result<(crate::state::Picker, Vec<CatalogEntry>), KernelError> {
    let attached = crate::bridge::attach(&cx.host, &cx.session).await?;
    let mut picker = crate::state::Picker {
        id: IntentId::mint().to_string(),
        all: name == "models",
        props: component::ModelPickerProps {
            filter: if name == "model" {
                args.into()
            } else {
                String::new()
            },
            effort: serde_json::from_value(attached.snapshot.config.kernel["thinking"].clone())
                .unwrap_or_default(),
            ..Default::default()
        },
        ..Default::default()
    };
    let models = cx.host.catalog(CatalogKind::Models).await?.entries;
    let providers = cx.host.catalog(CatalogKind::Providers).await?.entries;
    let rows = crate::rows::rows(&models, &providers, &picker.props.filter, picker.all);
    let current = crate::state::chat(&attached.snapshot).current;
    picker.props.selected = rows
        .iter()
        .find(|r| r.model() == Some(current.as_str()))
        .map(crate::rows::Row::key);
    if matches!(name, "login" | "logout")
        && let Some(provider) = providers
            .iter()
            .find(|p| p.id == args)
            .or_else(|| {
                providers
                    .iter()
                    .find(|p| current.split_once('/').is_some_and(|(id, _)| p.id == id))
            })
            .or_else(|| providers.first())
    {
        picker.sheet = Some(crate::sheet::Sheet::Manage {
            provider: provider.id.clone(),
            selected: 0,
            pending: None,
            confirm: false,
        });
    }
    Ok((picker, providers))
}

fn outcome(value: Value) -> Result<Value, KernelError> {
    if value["kind"] != "applied" {
        return Ok(value);
    }
    let Some(message) = value["message"].as_str() else {
        return Ok(value);
    };
    let text = if message.starts_with('⇄') {
        message.to_owned()
    } else if let Some(effort) = message.strip_prefix("thinking: ") {
        format!("⇄ thinking {effort}")
    } else {
        return Ok(value);
    };
    serde_json::to_value(CommandOutcome::Record {
        body: ItemBody::Notice {
            level: Level::Info,
            code: "model.choice".into(),
            text,
        },
    })
    .map_err(crate::error)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_a_bare_word_filters_and_a_provider_path_switches() {
        assert!(filters("opus"));
        assert!(!filters("fake/unlisted"));
        assert!(!filters("next"));
        assert!(!filters("pin fake/a"));
    }
    #[test]
    fn choice_receipts_are_recorded_and_other_outcomes_are_unchanged() {
        let value = outcome(json!({"kind":"applied","message":"⇄ small · here"})).unwrap();
        assert!(
            matches!(serde_json::from_value::<CommandOutcome>(value).unwrap(), CommandOutcome::Record { body: ItemBody::Notice {text, ..}} if text == "⇄ small · here")
        );
        let other = json!({"kind":"applied","message":"pinned"});
        assert_eq!(outcome(other.clone()).unwrap(), other);
    }
}
