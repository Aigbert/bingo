//! The owner opens its site before the palette's generic argument editor.
use bingo_sdk::*;
use serde_json::Value;
pub struct Describe;
#[async_trait::async_trait]
impl Interceptor for Describe {
    fn id(&self) -> &str {
        "models.describe"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher::at(catalogue::COMMAND_DESCRIBE)
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        _: &HookContext,
        value: Value,
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let mut value = next.run(value).await?;
        if matches!(
            value["name"].as_str(),
            Some("model" | "think" | "models" | "login" | "logout")
        ) {
            value["args"] = serde_json::to_value(ArgSpec::None).map_err(crate::error)?;
        }
        Ok(value)
    }
}
