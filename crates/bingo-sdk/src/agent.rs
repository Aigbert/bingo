//! An endpoint that owns its conversation and executes its own tools.

use crate::{
    ContentPart, Effort, ModelEvent, ModelInfo, ProviderError, SessionId, ToolOutput, ToolSpec,
    Verdict,
};
use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{path::PathBuf, pin::Pin, sync::Arc};

#[async_trait]
pub trait Agent: Send + Sync {
    fn id(&self) -> &str;
    async fn models(&self) -> Result<Vec<ModelInfo>, ProviderError>;
    async fn run(&self, turn: AgentTurn) -> Result<AgentStream, ProviderError>;
}

pub struct AgentTurn {
    pub session: SessionId,
    pub cwd: PathBuf,
    pub input: Vec<ContentPart>,
    pub tools: Vec<ToolSpec>,
    pub model: String,
    pub reasoning: Option<Effort>,
    pub judge: Arc<dyn Judge>,
}

pub type AgentStream = Pin<Box<dyn Stream<Item = Result<AgentEvent, ProviderError>> + Send>>;

#[derive(Clone, Debug, PartialEq)]
pub enum AgentEvent {
    Said(ModelEvent),
    Call(ExternalCall),
    Update(ExternalCall),
    Done {
        call: ExternalCall,
        output: ToolOutput,
    },
}

#[async_trait]
pub trait Judge: Send + Sync {
    async fn judge(&self, call: ExternalCall) -> Verdict;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CallKind {
    Read,
    Edit,
    Delete,
    Move,
    Search,
    Execute,
    Think,
    Fetch,
    Other,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CallLocation {
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExternalCall {
    pub id: String,
    pub kind: CallKind,
    pub title: String,
    pub locations: Vec<CallLocation>,
    pub raw: Value,
}

impl CallKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Edit => "edit",
            Self::Delete => "delete",
            Self::Move => "move",
            Self::Search => "search",
            Self::Execute => "execute",
            Self::Think => "think",
            Self::Fetch => "fetch",
            Self::Other => "other",
        }
    }

    pub fn writes(self) -> bool {
        matches!(self, Self::Edit | Self::Delete | Self::Move)
    }
}

impl ExternalCall {
    pub fn to_input(&self) -> Value {
        serde_json::json!({"title": self.title, "locations": self.locations, "raw": self.raw})
    }

    pub fn from_input(call: &crate::ToolCall) -> Option<Self> {
        let (_, kind) = call.name.rsplit_once(':')?;
        Some(Self {
            id: call.call_id.clone(),
            kind: serde_json::from_value(Value::String(kind.into())).ok()?,
            title: call.input.get("title")?.as_str()?.into(),
            locations: serde_json::from_value(call.input.get("locations")?.clone()).ok()?,
            raw: call.input.get("raw")?.clone(),
        })
    }

    pub fn tool_call(&self, agent: &str) -> crate::ToolCall {
        crate::ToolCall {
            call_id: self.id.clone(),
            name: format!("{agent}:{}", self.kind.as_str()),
            input: self.to_input(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolCall;

    #[test]
    fn external_call_round_trips_the_journal_fixture() {
        let fixture: ToolCall =
            serde_json::from_str(include_str!("../fixtures/agent/edit.json")).unwrap();
        let external = ExternalCall::from_input(&fixture).unwrap();
        assert_eq!(external.kind, CallKind::Edit);
        assert_eq!(external.id, "edit-1");
        assert_eq!(external.title, "Edit note.md");
        assert_eq!(external.locations[0].path, PathBuf::from("note.md"));
        assert_eq!(external.to_input(), fixture.input);
        assert_eq!(external.tool_call("fake"), fixture);
    }
}
