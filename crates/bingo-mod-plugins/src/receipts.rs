//! Switches acknowledge only after their owning command succeeds.
use bingo_sdk::*;
use serde_json::Value;
pub struct Receipts;
#[async_trait::async_trait]
impl Interceptor for Receipts {
    fn id(&self) -> &str {
        "plugins.receipts"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher::at(catalogue::COMMAND_RUN)
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        _: &HookContext,
        value: Value,
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let said = switch_toast(&value);
        let result = next.run(value).await?;
        match said {
            Some(said) if result["kind"] == "applied" => {
                serde_json::to_value(said.outcome(said.plain())).map_err(crate::error)
            }
            _ => Ok(result),
        }
    }
}
fn switch_toast(value: &Value) -> Option<toast::Say> {
    let run: catalogue::CommandRun = serde_json::from_value(value.clone()).ok()?;
    if !run.surface.is_some_and(render::SurfaceName::shows_sheets)
        || !matches!(run.name.as_str(), "plugins" | "mcp")
    {
        return None;
    }
    let mut words = run.args.split_whitespace();
    let on = match words.next()? {
        "enable" => true,
        "disable" => false,
        _ => return None,
    };
    let id = words.next()?;
    if words.next().is_some() {
        return None;
    }
    let name = id.strip_prefix("plugins:").unwrap_or(id);
    let (word, ink) = if on { ("on", "good") } else { ("off", "muted") };
    Some(toast::Say::new([
        view::TextChild::from(pick::text(format!("{name} {word}"), ink)),
        "from the next turn".into(),
    ]))
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn switches_use_the_prototypes_exact_words_only_on_terminal() {
        let said = switch_toast(
            &json!({"command":"plugins","args":"disable tool-fs","surface":"terminal"}),
        )
        .unwrap();
        assert_eq!(said.plain(), "tool-fs off · from the next turn");
        assert_eq!(said.parts[0], pick::text("tool-fs off", "muted").into());
        let on = switch_toast(
            &json!({"command":"plugins","args":"enable tool-fs","surface":"terminal"}),
        )
        .unwrap();
        assert_eq!(on.parts[0], pick::text("tool-fs on", "good").into());
        assert!(
            switch_toast(&json!({"command":"plugins","args":"disable tool-fs","surface":"text"}))
                .is_none()
        );
        assert!(
            switch_toast(
                &json!({"command":"plugins","args":"trust tool-fs full","surface":"terminal"})
            )
            .is_none()
        );
    }
}
