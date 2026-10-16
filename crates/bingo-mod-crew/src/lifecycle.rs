use bingo_sdk::*;
use serde_json::{Value, json};

pub struct Disable(pub crate::Shared);

#[async_trait::async_trait]
impl Interceptor for Disable {
    fn id(&self) -> &str {
        "crew:disable"
    }

    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"command":"plugins"}),
            ..LinkMatcher::at(catalogue::COMMAND_RUN)
        }
    }

    async fn chain(
        &self,
        _: &catalogue::Entry,
        cx: &HookContext,
        value: Value,
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let run: catalogue::CommandRun =
            serde_json::from_value(value.clone()).map_err(crate::error)?;
        if run.name != "plugins"
            || run.args.split_whitespace().collect::<Vec<_>>() != ["disable", crate::ID]
        {
            return next.run(value).await;
        }
        let reply = next.run(value).await?;
        if matches!(
            serde_json::from_value::<CommandOutcome>(reply.clone()),
            Ok(CommandOutcome::Applied { .. })
        ) && off(cx).await?
        {
            clear(&self.0, &cx.host).await?;
        }
        Ok(reply)
    }
}

async fn off(cx: &HookContext) -> Result<bool, KernelError> {
    let catalog = cx
        .host
        .catalog_for(&cx.session, CatalogKind::Plugins)
        .await?;
    Ok(catalog
        .entries
        .iter()
        .any(|entry| entry.id == crate::ID && entry.meta["state"]["kind"] == "off"))
}

async fn clear(shared: &crate::Shared, host: &HostHandle) -> Result<(), KernelError> {
    let sessions = {
        let mut panels = shared.lock().map_err(crate::error)?;
        std::mem::take(&mut *panels).into_keys().collect::<Vec<_>>()
    };
    for session in sessions {
        host.signal(&session, crate::ID, crate::REQUEST, Value::Null)
            .await?;
    }
    Ok(())
}
