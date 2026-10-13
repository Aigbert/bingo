//! Dispatch links. A continuation is consumed once; composing precedes measurement.

use crate::*;
use async_trait::async_trait;
use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod stream;
pub use stream::StepStream;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum LinkPoint {
    #[serde(rename = "prompt.submit")]
    PromptSubmit,
    #[serde(rename = "tool.check")]
    ToolCheck,
    #[serde(rename = "tool.call")]
    ToolCall,
    #[serde(rename = "turn.step")]
    TurnStep,
    #[serde(rename = "prompt.compose")]
    PromptCompose,
    #[serde(rename = "session.compact")]
    SessionCompact,
    #[serde(rename = "turn.stop")]
    TurnStop,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LinkMatcher {
    #[serde(default)]
    pub points: Vec<LinkPoint>,
    /// Exact tool name or a trailing `*` prefix, as with hooks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Tier {
    #[default]
    Tighten,
    Rewrite,
    Full,
}

/// Only the kernel reads this grant from the user settings layer.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PluginTrust {
    pub tier: Tier,
    #[serde(default)]
    pub uses: Vec<String>,
}

/// The continuation cannot be cloned or called twice.
pub struct Next<'a, I, O>(Box<dyn FnOnce(I) -> BoxFuture<'a, Result<O, KernelError>> + Send + 'a>);

impl<'a, I, O> Next<'a, I, O> {
    pub fn new(run: impl FnOnce(I) -> BoxFuture<'a, Result<O, KernelError>> + Send + 'a) -> Self {
        Self(Box::new(run))
    }

    pub async fn run(self, input: I) -> Result<O, KernelError> {
        (self.0)(input).await
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SubmitResult {
    Accepted(Input),
    Rejected(KernelError),
    Redirect { session: SessionId, input: Input },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CheckResult {
    pub call: ToolCall,
    pub decision: Decision,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompactInput {
    pub items: Vec<Item>,
    pub usage: ContextUsage,
    pub request: ModelRequest,
    pub reason: CompactReason,
    pub failures: u32,
    pub keep_budget: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StopResult {
    Complete,
    Continue { origin: String, reason: String },
}

#[async_trait]
pub trait Interceptor: Send + Sync {
    fn id(&self) -> &str;
    fn matcher(&self) -> LinkMatcher;

    async fn prompt_submit(
        &self,
        _cx: &HookContext,
        input: Input,
        next: Next<'_, Input, SubmitResult>,
    ) -> Result<SubmitResult, KernelError> {
        next.run(input).await
    }
    async fn tool_check(
        &self,
        _cx: &HookContext,
        input: ToolCall,
        next: Next<'_, ToolCall, CheckResult>,
    ) -> Result<CheckResult, KernelError> {
        next.run(input).await
    }
    async fn tool_call(
        &self,
        _cx: &HookContext,
        input: ToolCall,
        next: Next<'_, ToolCall, ToolOutput>,
    ) -> Result<ToolOutput, KernelError> {
        next.run(input).await
    }
    async fn turn_step(
        &self,
        _cx: &HookContext,
        input: ModelRequest,
        next: Next<'_, ModelRequest, Result<StepStream, ProviderError>>,
    ) -> Result<Result<StepStream, ProviderError>, KernelError> {
        next.run(input).await
    }
    /// The final composed request is measured after this chain returns.
    async fn prompt_compose(
        &self,
        _cx: &HookContext,
        input: ModelRequest,
        next: Next<'_, ModelRequest, ModelRequest>,
    ) -> Result<ModelRequest, KernelError> {
        next.run(input).await
    }
    async fn session_compact(
        &self,
        _cx: &HookContext,
        input: CompactInput,
        next: Next<'_, CompactInput, Result<Compaction, CompactError>>,
    ) -> Result<Result<Compaction, CompactError>, KernelError> {
        next.run(input).await
    }
    async fn turn_stop(
        &self,
        _cx: &HookContext,
        input: (),
        next: Next<'_, (), StopResult>,
    ) -> Result<StopResult, KernelError> {
        next.run(input).await
    }
}
