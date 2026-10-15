//! The picker is where these commands choose their argument, so a palette
//! runs them bare; their argument hints stay for every other surface.
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
            value["opens"] = Value::Bool(true);
        }
        Ok(value)
    }
}
