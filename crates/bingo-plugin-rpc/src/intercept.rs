//! Two remote phases around a host continuation.
use crate::wire::HookSite;
use bingo_sdk::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LinkSpec {
    pub point: LinkPoint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(default)]
    pub after: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Before<I> {
    pub chain: String,
    pub tier: Tier,
    pub site: HookSite,
    pub input: I,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Proceed<I, O> {
    Next(I),
    Answer(O),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct After<O> {
    pub chain: String,
    pub result: O,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultValue<O> {
    pub result: O,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Cancel {
    pub chain: String,
}

pub use bingo_sdk::catalogue::ToolCall as ToolEvent;

use crate::connection::Connection;
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use std::sync::Arc;

pub fn point_name(point: LinkPoint) -> &'static str {
    point.name.as_ref()
}

pub struct Remote {
    id: String,
    matcher: LinkMatcher,
    tool: Option<String>,
    after: bool,
    connection: Arc<Connection>,
}
impl Remote {
    pub fn new(plugin: &str, spec: LinkSpec, connection: Arc<Connection>) -> Self {
        Self {
            id: format!("{plugin}:{}", point_name(spec.point)),
            matcher: LinkMatcher::at(spec.point),
            tool: spec.tool,
            after: spec.after,
            connection,
        }
    }

    pub fn subscription(plugin: &str, matcher: LinkMatcher, connection: Arc<Connection>) -> Self {
        Self {
            id: format!("{plugin}:catalogue"),
            matcher,
            tool: None,
            after: true,
            connection,
        }
    }

    async fn phase<I: Serialize, O: DeserializeOwned>(
        &self,
        method: &str,
        input: I,
    ) -> Result<O, KernelError> {
        let params = serde_json::to_value(input).map_err(failed)?;
        let value = tokio::time::timeout(
            crate::deadline::HOOK,
            self.connection.request(method, params),
        )
        .await
        .map_err(|_| failed("interceptor phase deadline exceeded"))?
        .map_err(|error| failed(error.message))?;
        serde_json::from_value(value).map_err(failed)
    }

    async fn cross<I, O>(
        &self,
        entry: &catalogue::Entry,
        cx: &HookContext,
        input: I,
        next: Next<'_, I, O>,
    ) -> Result<O, KernelError>
    where
        I: Serialize + DeserializeOwned + Send,
        O: Serialize + DeserializeOwned + Send,
    {
        let chain = self.connection.next_call();
        let _context = self.connection.hold_chain(&chain);
        let mut cancel = CancelOnDrop {
            connection: self.connection.clone(),
            chain: Some(chain.clone()),
        };
        let method = format!("intercept/{}", entry.name);
        let answer: Proceed<I, O> = self
            .phase(
                &method,
                Before {
                    chain: chain.clone(),
                    tier: next.tier(),
                    site: HookSite::from(cx),
                    input,
                },
            )
            .await?;
        let result = match answer {
            Proceed::Answer(output) => output,
            Proceed::Next(input) => {
                let result = next.run(input).await?;
                if self.after {
                    self.phase::<_, ResultValue<O>>(
                        &format!("{method}/after"),
                        After { chain, result },
                    )
                    .await?
                    .result
                } else {
                    result
                }
            }
        };
        cancel.chain = None;
        Ok(result)
    }
}

struct CancelOnDrop {
    connection: Arc<Connection>,
    chain: Option<String>,
}
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if let Some(chain) = self.chain.take() {
            self.connection
                .notify_now("intercept/cancel", serde_json::json!({"chain":chain}));
        }
    }
}
fn failed(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::Internal, error.to_string())
}

#[async_trait]
impl Interceptor for Remote {
    fn id(&self) -> &str {
        &self.id
    }
    fn matcher(&self) -> LinkMatcher {
        self.matcher.clone()
    }
    async fn observe(&self, entry: &catalogue::Entry, cx: &HookContext, input: &Value) {
        self.connection.notify("hook/observe", serde_json::json!({
            "id":self.id, "site":HookSite::from(cx), "point":"catalogue", "event":entry.name, "payload":input,
        })).await;
    }
    async fn chain(
        &self,
        entry: &catalogue::Entry,
        cx: &HookContext,
        input: Value,
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        if let Some(pattern) = &self.tool {
            let tool = input
                .get("tool")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if !pattern
                .strip_suffix('*')
                .map_or(pattern == tool, |prefix| tool.starts_with(prefix))
            {
                return next.run(input).await;
            }
        }
        self.cross(entry, cx, input, next).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalogue_tool_fields_and_completion_spelling() {
        let event: ToolEvent = serde_json::from_value(
            serde_json::json!({"tool":"Bash","tool_use_id":"c1","command":"ls"}),
        )
        .unwrap();
        assert_eq!(event.input["command"], "ls");
        assert_eq!(
            serde_json::to_value(bingo_sdk::catalogue::TURN_STOP).unwrap(),
            "turn.stop"
        );
        let next = Proceed::<_, ToolOutput>::Next(event);
        let value = serde_json::to_value(next).unwrap();
        assert_eq!(value["next"]["tool_use_id"], "c1");
        assert!(value["next"].get("input").is_none());
        let result = serde_json::to_value(ResultValue {
            result: ToolOutput::error("failed"),
        })
        .unwrap();
        assert_eq!(result["result"]["isError"], true);
    }
}

#[cfg(test)]
mod catalogue_contract {
    #[test]
    fn initialization_retains_open_catalogue_subscriptions() {
        let value = serde_json::json!({"protocol":2,"name":"watch","version":"1",
            "subscriptions":[{"patterns":["mcp.*"],"prefilter":null}]});
        let result: crate::wire::InitializeResult = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(result).unwrap()["subscriptions"],
            value["subscriptions"]
        );
        assert!(crate::wire::capabilities().contains("intercept/*"));
    }
}
