//! Dispatch links. A continuation is consumed once; composing precedes measurement.

use crate::*;
use async_trait::async_trait;
use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Wall bound for an external dispatch link (ADR-0032, ADR-0074).
pub const HOOK_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

mod json;
mod stream;
pub use stream::StepStream;

pub type LinkPoint = &'static crate::catalogue::Entry;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LinkMatcher {
    #[serde(default)]
    pub patterns: Vec<crate::catalogue::Pattern>,
    #[serde(default)]
    pub prefilter: serde_json::Value,
}
impl LinkMatcher {
    pub fn at(entry: LinkPoint) -> Self {
        Self {
            patterns: crate::catalogue::Pattern::parse(&entry.name)
                .into_iter()
                .collect(),
            prefilter: serde_json::Value::Null,
        }
    }
    pub fn matches(&self, entry: &crate::catalogue::Entry, input: &serde_json::Value) -> bool {
        (self.patterns.is_empty() || self.patterns.iter().any(|p| p.matches(&entry.name)))
            && (self.prefilter.is_null() || crate::catalogue::prefilter(&self.prefilter, input))
    }
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

/// A process identity accompanies every dynamically supplied link.
#[derive(Clone)]
pub struct ExternalInterceptor {
    pub plugin: String,
    pub link: std::sync::Arc<dyn Interceptor>,
}

#[async_trait]
pub trait InterceptorSource: Send + Sync {
    fn id(&self) -> &str;
    async fn interceptors(&self) -> Vec<ExternalInterceptor>;
}

/// The continuation cannot be cloned or called twice.
///
/// ```compile_fail
/// use bingo_sdk::Next;
/// async fn twice(next: Next<'_, (), ()>) {
///     let _ = next.run(()).await;
///     let _ = next.run(()).await;
/// }
/// ```
pub struct Next<'a, I, O> {
    run: Box<dyn FnOnce(I) -> BoxFuture<'a, Result<O, KernelError>> + Send + 'a>,
    tier: Tier,
    phase: Option<tokio::sync::watch::Sender<bool>>,
}

impl<'a, I, O> Next<'a, I, O> {
    pub fn new(run: impl FnOnce(I) -> BoxFuture<'a, Result<O, KernelError>> + Send + 'a) -> Self {
        Self::at(Tier::Full, run)
    }

    pub fn at(
        tier: Tier,
        run: impl FnOnce(I) -> BoxFuture<'a, Result<O, KernelError>> + Send + 'a,
    ) -> Self {
        Self {
            run: Box::new(run),
            tier,
            phase: None,
        }
    }

    pub fn tracked(mut self, phase: tokio::sync::watch::Sender<bool>) -> Self {
        self.phase = Some(phase);
        self
    }

    pub fn tier(&self) -> Tier {
        self.tier
    }

    pub async fn run(self, input: I) -> Result<O, KernelError> {
        self.run_then(input, std::future::ready).await
    }

    /// Keep collecting a streaming continuation outside the caller's phase clock.
    pub async fn run_then<R, F: std::future::Future<Output = R>>(
        self,
        input: I,
        finish: impl FnOnce(O) -> F,
    ) -> Result<R, KernelError> {
        if let Some(phase) = &self.phase {
            phase.send_replace(false);
        }
        let _resume = Resume(self.phase);
        Ok(finish((self.run)(input).await?).await)
    }
}

struct Resume(Option<tokio::sync::watch::Sender<bool>>);
impl Drop for Resume {
    fn drop(&mut self) {
        if let Some(phase) = &self.0 {
            phase.send_replace(true);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SubmitResult {
    Accepted(Input),
    Rejected(KernelError),
    Redirect { session: SessionId, input: Input },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckResult {
    pub call: ToolCall,
    pub decision: Decision,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompactInput {
    pub items: Vec<Item>,
    pub usage: ContextUsage,
    pub request: ModelRequest,
    pub reason: CompactReason,
    pub failures: u32,
    pub keep_budget: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum StopResult {
    Complete,
    Continue { origin: String, reason: String },
}

#[async_trait]
pub trait Interceptor: Send + Sync {
    fn id(&self) -> &str;
    fn matcher(&self) -> LinkMatcher;

    async fn chain(
        &self,
        _entry: &catalogue::Entry,
        _cx: &HookContext,
        input: serde_json::Value,
        next: Next<'_, serde_json::Value, serde_json::Value>,
    ) -> Result<serde_json::Value, KernelError> {
        next.run(input).await
    }
    async fn observe(
        &self,
        _entry: &catalogue::Entry,
        _cx: &HookContext,
        _input: &serde_json::Value,
    ) {
    }

    async fn prompt_submit(
        &self,
        cx: &HookContext,
        input: Input,
        next: Next<'_, Input, SubmitResult>,
    ) -> Result<SubmitResult, KernelError> {
        json::bridge(self, catalogue::PROMPT_SUBMIT, cx, input, next).await
    }
    /// Tool identity and call id are pinned; links may rewrite `input`.
    async fn tool_check(
        &self,
        cx: &HookContext,
        input: ToolCall,
        next: Next<'_, ToolCall, CheckResult>,
    ) -> Result<CheckResult, KernelError> {
        json::tool(self, catalogue::TOOL_CHECK, cx, input, next).await
    }
    async fn tool_call(
        &self,
        cx: &HookContext,
        input: ToolCall,
        next: Next<'_, ToolCall, ToolOutput>,
    ) -> Result<ToolOutput, KernelError> {
        json::tool(self, catalogue::TOOL_CALL, cx, input, next).await
    }
    async fn turn_step(
        &self,
        cx: &HookContext,
        input: ModelRequest,
        next: Next<'_, ModelRequest, Result<StepStream, ProviderError>>,
    ) -> Result<Result<StepStream, ProviderError>, KernelError> {
        json::step(self, cx, input, next).await
    }
    /// The final composed request is measured after this chain returns.
    async fn prompt_compose(
        &self,
        cx: &HookContext,
        input: ModelRequest,
        next: Next<'_, ModelRequest, ModelRequest>,
    ) -> Result<ModelRequest, KernelError> {
        json::bridge(self, catalogue::PROMPT_COMPOSE, cx, input, next).await
    }
    async fn session_compact(
        &self,
        cx: &HookContext,
        input: CompactInput,
        next: Next<'_, CompactInput, Result<Compaction, CompactError>>,
    ) -> Result<Result<Compaction, CompactError>, KernelError> {
        json::bridge(self, catalogue::SESSION_COMPACT, cx, input, next).await
    }
    async fn turn_stop(
        &self,
        cx: &HookContext,
        input: (),
        next: Next<'_, (), StopResult>,
    ) -> Result<StopResult, KernelError> {
        json::bridge(self, catalogue::TURN_STOP, cx, input, next).await
    }
}
