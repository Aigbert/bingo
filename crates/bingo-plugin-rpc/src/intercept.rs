//! Two remote phases around a host continuation.
use crate::wire::HookSite;
use bingo_sdk::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

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

/// Catalogue spelling: tool input fields live beside the pinned identity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolEvent {
    pub tool: String,
    pub tool_use_id: String,
    #[serde(flatten)]
    pub input: BTreeMap<String, Value>,
}

use crate::connection::Connection;
use async_trait::async_trait;
use futures::{FutureExt, StreamExt};
use serde::de::DeserializeOwned;
use std::sync::Arc;

pub fn point_name(point: LinkPoint) -> &'static str {
    match point {
        LinkPoint::PromptSubmit => "prompt.submit",
        LinkPoint::ToolCheck => "tool.check",
        LinkPoint::ToolCall => "tool.call",
        LinkPoint::TurnStep => "turn.step",
        LinkPoint::PromptCompose => "prompt.compose",
        LinkPoint::SessionCompact => "session.compact",
        LinkPoint::TurnStop => "turn.complete",
    }
}

pub struct Remote {
    id: String,
    spec: LinkSpec,
    connection: Arc<Connection>,
}
impl Remote {
    pub fn new(plugin: &str, spec: LinkSpec, connection: Arc<Connection>) -> Self {
        Self {
            id: format!("{plugin}:{}", point_name(spec.point)),
            spec,
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
        cx: &HookContext,
        input: I,
        next: Next<'_, I, O>,
    ) -> Result<O, KernelError>
    where
        I: Serialize + DeserializeOwned + Send,
        O: Serialize + DeserializeOwned + Send,
    {
        let chain = self.connection.next_call();
        let mut cancel = CancelOnDrop {
            connection: self.connection.clone(),
            chain: Some(chain.clone()),
        };
        let method = format!("intercept/{}", point_name(self.spec.point));
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
                if self.spec.after {
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

impl TryFrom<ToolCall> for ToolEvent {
    type Error = KernelError;
    fn try_from(call: ToolCall) -> Result<Self, Self::Error> {
        let input = serde_json::from_value(call.input).map_err(failed)?;
        Ok(Self {
            tool: call.name,
            tool_use_id: call.call_id,
            input,
        })
    }
}
impl From<ToolEvent> for ToolCall {
    fn from(event: ToolEvent) -> Self {
        Self {
            name: event.tool,
            call_id: event.tool_use_id,
            input: Value::Object(event.input.into_iter().collect()),
        }
    }
}

#[async_trait]
impl Interceptor for Remote {
    fn id(&self) -> &str {
        &self.id
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            points: vec![self.spec.point],
            tool: self.spec.tool.clone(),
        }
    }
    async fn prompt_submit(
        &self,
        cx: &HookContext,
        input: Input,
        next: Next<'_, Input, SubmitResult>,
    ) -> Result<SubmitResult, KernelError> {
        self.cross(cx, input, next).await
    }
    async fn tool_check(
        &self,
        cx: &HookContext,
        input: ToolCall,
        next: Next<'_, ToolCall, CheckResult>,
    ) -> Result<CheckResult, KernelError> {
        let tier = next.tier();
        let continuation = Next::at(tier, move |event: ToolEvent| {
            async move { next.run(event.into()).await }.boxed()
        });
        self.cross(cx, input.try_into()?, continuation).await
    }
    async fn tool_call(
        &self,
        cx: &HookContext,
        input: ToolCall,
        next: Next<'_, ToolCall, ToolOutput>,
    ) -> Result<ToolOutput, KernelError> {
        let tier = next.tier();
        let continuation = Next::at(tier, move |event: ToolEvent| {
            async move { next.run(event.into()).await }.boxed()
        });
        self.cross(cx, input.try_into()?, continuation).await
    }
    async fn prompt_compose(
        &self,
        cx: &HookContext,
        input: ModelRequest,
        next: Next<'_, ModelRequest, ModelRequest>,
    ) -> Result<ModelRequest, KernelError> {
        self.cross(cx, input, next).await
    }
    async fn session_compact(
        &self,
        cx: &HookContext,
        input: CompactInput,
        next: Next<'_, CompactInput, Result<Compaction, CompactError>>,
    ) -> Result<Result<Compaction, CompactError>, KernelError> {
        self.cross(cx, input, next).await
    }
    async fn turn_stop(
        &self,
        cx: &HookContext,
        input: (),
        next: Next<'_, (), StopResult>,
    ) -> Result<StopResult, KernelError> {
        self.cross(cx, input, next).await
    }
    async fn turn_step(
        &self,
        cx: &HookContext,
        input: ModelRequest,
        next: Next<'_, ModelRequest, Result<StepStream, ProviderError>>,
    ) -> Result<Result<StepStream, ProviderError>, KernelError> {
        let held = Arc::new(std::sync::Mutex::new(None));
        let hold = held.clone();
        let continuation = Next::at(next.tier(), move |request| {
            async move {
                next.run_then(request, |original| async move {
                    let events: Result<Vec<ModelEvent>, ProviderError> = match original.clone() {
                        Ok(stream) => stream.collect::<Vec<_>>().await.into_iter().collect(),
                        Err(error) => Err(error),
                    };
                    *hold.lock().unwrap_or_else(|e| e.into_inner()) =
                        Some((events.clone(), original));
                    events
                })
                .await
            }
            .boxed()
        });
        let result: Result<Vec<ModelEvent>, ProviderError> =
            self.cross(cx, input, continuation).await?;
        let original = held.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some((events, original)) = original
            && result == events
        {
            return Ok(original);
        }
        Ok(result.map(|events| {
            StepStream::new(Box::pin(futures::stream::iter(events.into_iter().map(Ok))))
        }))
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
            serde_json::to_value(LinkPoint::TurnStop).unwrap(),
            "turn.complete"
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
