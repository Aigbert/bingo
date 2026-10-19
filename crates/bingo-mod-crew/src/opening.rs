//! Bare `/agents` in a surface that shows sheets opens the crew sheet; its
//! plain table stays the fallback, and every other caller keeps it.
use bingo_sdk::*;
use serde_json::{Value, json};

/// The command whose bare form opens the sheet.
pub const OPENS: &str = "agents";

pub struct Opening(pub crate::Shared);

#[async_trait::async_trait]
impl Interceptor for Opening {
    fn id(&self) -> &str {
        OPENS
    }

    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"command":OPENS,"args":""}),
            ..LinkMatcher::at(catalogue::COMMAND_RUN)
        }
    }

    async fn chain(
        &self,
        _: &catalogue::Entry,
        cx: &HookContext,
        input: Value,
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let run: catalogue::CommandRun =
            serde_json::from_value(input.clone()).map_err(crate::error)?;
        if run.name != OPENS
            || !run.args.trim().is_empty()
            || !run.surface.is_some_and(render::SurfaceName::shows_sheets)
        {
            return next.run(input).await;
        }
        let plain = next.run(input).await?;
        let fallback = match serde_json::from_value(plain.clone()) {
            Ok(CommandOutcome::View { view }) => Some(view.fold_markdown()),
            Ok(CommandOutcome::Applied { message }) => message,
            _ => return Ok(plain),
        };
        let outcome = crate::command::open(&self.0, &cx.host, &cx.session, fallback).await?;
        serde_json::to_value(outcome).map_err(crate::error)
    }
}
