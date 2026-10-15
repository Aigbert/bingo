//! The agents plugin owns the noun and the same spawn used by its tool.
use crate::{
    definition::Definition,
    library::Definitions,
    spawn::{SpawnAgentTool, SpawnArgs},
};
use bingo_sdk::*;
use serde_json::{Value, json};
use std::sync::{Arc, OnceLock, Weak};

pub const KEY: ServiceKey<AgentService> = ServiceKey::new("bingo.agents.agent");
tokio::task_local! { static CONTEXT: Arc<ToolContext>; }
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct Spawn {
    session: SessionId,
    args: SpawnArgs,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct Register {
    definition: Definition,
}
pub struct AgentService {
    host: OnceLock<Weak<dyn HostApi>>,
    env: Arc<Env>,
    tool: SpawnAgentTool,
}
impl std::fmt::Debug for AgentService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentService")
    }
}
impl AgentService {
    pub fn new(
        env: Env,
        reports: Arc<crate::report::Reports>,
        definitions: Arc<Definitions>,
    ) -> Self {
        let mut tool = SpawnAgentTool::new(reports);
        tool.definitions = definitions;
        Self {
            host: OnceLock::new(),
            env: Arc::new(env),
            tool,
        }
    }
    pub fn bind(&self, host: &HostHandle) {
        let _ = self.host.set(Arc::downgrade(&host.0));
    }
    pub(crate) fn host(&self) -> Result<HostHandle, ServiceError> {
        self.host
            .get()
            .and_then(Weak::upgrade)
            .map(HostHandle)
            .ok_or_else(|| ServiceError("agent host is unavailable".into()))
    }
    async fn context(&self, session: &SessionId) -> Result<Arc<ToolContext>, ServiceError> {
        if let Ok(cx) = CONTEXT.try_with(Clone::clone) {
            if cx.session != *session {
                return Err(ServiceError("agent caller session is pinned".into()));
            }
            return Ok(cx);
        }
        let host = self.host()?;
        let summary = host
            .sessions(SessionFilter::default())
            .await
            .map_err(failed)?
            .into_iter()
            .find(|s| s.id == *session)
            .ok_or_else(|| ServiceError("agent session is unavailable".into()))?;
        Ok(Arc::new(ToolContext {
            call_id: String::new(),
            session: session.clone(),
            turn: TurnId::mint(),
            item: ItemId::mint(),
            cwd: summary.cwd.into(),
            cancel: CancellationToken::new(),
            env: self.env.clone(),
            host,
            call: Arc::new(NoCall),
        }))
    }
}
#[async_trait::async_trait]
impl WireService for AgentService {
    fn face(&self) -> Option<Face> {
        let mut face = Face::new("agent")
            .method::<Spawn, ToolOutput>("spawn", true)
            .method::<catalogue::Session, Vec<SessionSummary>>("list", false)
            .method::<Register, ()>("register", false);
        for method in &mut face.methods {
            method.entry.pinned = &["/session"];
        }
        Some(face)
    }
    async fn call(&self, method: &str, input: Value) -> Result<Value, ServiceError> {
        match method {
            "spawn" => {
                let input: Spawn = serde_json::from_value(input).map_err(failed)?;
                let cx = self.context(&input.session).await?;
                Ok(json!(
                    self.tool
                        .execute(json!(input.args), &cx)
                        .await
                        .map_err(failed)?
                ))
            }
            "list" => {
                let input: catalogue::Session = serde_json::from_value(input).map_err(failed)?;
                Ok(json!(
                    crate::names::agents(&self.host()?, &input.session)
                        .await
                        .map_err(failed)?
                ))
            }
            "register" => {
                let input: Register = serde_json::from_value(input).map_err(failed)?;
                self.tool.definitions.register(input.definition)?;
                Ok(Value::Null)
            }
            _ => Err(ServiceError("agent speaks spawn, list and register".into())),
        }
    }
}
pub async fn invoke(
    wire: Arc<dyn WireService>,
    input: Value,
    cx: &ToolContext,
) -> Result<ToolOutput, ToolError> {
    let input = json!({"session":cx.session,"args":input});
    let context = Arc::new(ToolContext {
        call_id: cx.call_id.clone(),
        session: cx.session.clone(),
        turn: cx.turn.clone(),
        item: cx.item.clone(),
        cwd: cx.cwd.clone(),
        cancel: cx.cancel.clone(),
        env: cx.env.clone(),
        host: cx.host.clone(),
        call: cx.call.clone(),
    });
    let value = CONTEXT
        .scope(context, wire.call("spawn", input))
        .await
        .map_err(|e| ToolError::Failed(e.to_string()))?;
    serde_json::from_value(value).map_err(|e| ToolError::Failed(e.to_string()))
}
fn failed(error: impl std::fmt::Display) -> ServiceError {
    ServiceError(error.to_string())
}
struct NoCall;
#[async_trait::async_trait]
impl Prompter for NoCall {
    async fn ask(&self, _: InteractionKind, _: Vec<AnswerSpec>) -> Result<Answer, KernelError> {
        Err(KernelError::new(
            ErrorCode::NotReady,
            "agent noun has no tool interaction",
        ))
    }
}
#[async_trait::async_trait]
impl ToolHost for NoCall {
    fn progress(&self, _: &ItemId, _: String) {}
    async fn record(&self, _: ItemBody) -> Result<ItemId, KernelError> {
        Err(KernelError::new(
            ErrorCode::NotReady,
            "agent noun has no tool item",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn registration_lives_in_this_hosts_agent_library() {
        let home = tempfile::tempdir().unwrap();
        let definitions = Arc::new(Definitions::default());
        let service = AgentService::new(
            Env::rooted(home.path()),
            Arc::default(),
            definitions.clone(),
        );
        let definition = Definition::parse("reviewer", "Check the implementation.");
        assert_eq!(
            service
                .call("register", json!({"definition":definition}))
                .await
                .unwrap(),
            Value::Null
        );
        let loaded = definitions.load(&Env::rooted(home.path()), home.path());
        assert_eq!(loaded, [definition]);
        assert!(
            Definitions::default()
                .load(&Env::rooted(home.path()), home.path())
                .is_empty()
        );
    }
}
