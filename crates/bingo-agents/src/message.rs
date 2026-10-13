//! What one session says to another. One tool, one delivery: `SendMessage`
//! wakes an idle target and reaches a busy one mid-run (ADR-0024 §2). A post
//! into a room goes through the same door and is weighed there first: a closed
//! room takes none (ADR-0053 §4), and one written behind the room's head is
//! handed back with what it missed (ADR-0025) — the draft still in the call
//! that made it, so `again: true` posts those words as they stand
//! (ADR-0053 §6).

use std::path::Path;

use async_trait::async_trait;
use bingo_sdk::{
    Delivery, Driver, Input, IntentId, Origin, Subject,
    Tool, ToolContext, ToolError, ToolOutput, ToolSpec, ToolTraits, View, input_schema,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use crate::names;
use bingo_services::room_post::{self, Posted, Request, Said};

/// The surface an agent's messages come from; a person's say `tui` or `print`.
pub const SURFACE: &str = "agent";

/// The tool a message is written through, as a journal records the name of the
/// call that wrote one.
pub const SEND_MESSAGE: &str = "SendMessage";

/// Who a message is from. The kernel's fold turns a principal into a
/// `[from <name>]` line above the text, so a name is never written into the
/// text itself.
pub fn origin(principal: Option<String>) -> Origin {
    Origin {
        surface: SURFACE.into(),
        principal,
        conversation: None,
    }
}

const DESCRIPTION: &str = "\
Write to another session: an agent you started, a teammate beside you — one \
the same agent started, which `ListAgents` names under `Beside you` — \
`parent`, the agent that started you, or `#room`, a conversation every member \
of which reads it. The message arrives whatever the target is doing: an idle \
one takes it up as its next turn, one that is working reads it mid-run. A \
direct message asks for nothing back: say what you have and go on. When you \
need an answer from someone, ask in a room with `@name` — a mention is owed an \
answer, a direct message is not. `to` is the name `SpawnAgent` gave back, a \
teammate's name, `parent`, or a room's `#name`. A room hands a post back when \
somebody spoke while you were writing: read what they said, then post the same \
words again with `again: true` and no `text` — the draft is already in your \
call, so repeating it costs nothing to write.";

/// What the caller is told. A log session has no turns (ADR-0011 §1), so a
/// receipt about one is not true of a room: the post is the whole of what
/// happened.
fn receipt(to: &str, driver: Driver) -> String {
    match driver {
        Driver::Log => format!("Posted to {to}."),
        Driver::Model => {
            format!(
                "Sent to {to}; it takes it up as its next turn, or reads it mid-run if it is already working."
            )
        }
    }
}

/// The same delivery a person reads (ADR-0013, the block lane): where it
/// went, whose name it arrives under, and when it will be read. The model has
/// the sentence above; this is the receipt beside it.
fn card(to: &str, from: &str, driver: Driver) -> View {
    View::KeyValue {
        rows: vec![
            ("to".into(), to.to_string()),
            ("from".into(), from.to_string()),
            ("read".into(), read(driver).to_string()),
        ],
    }
}

/// When it will be read, in the few cells a card row has for it.
fn read(driver: Driver) -> &'static str {
    match driver {
        Driver::Log => "at each seat's next turn; an @ wakes now",
        Driver::Model => "as its next turn, or mid-run if it is working",
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MessageArgs {
    /// Who to write to: the name `SpawnAgent` gave back, a teammate's name,
    /// `parent` for the agent that started you, or `#name` for a room.
    pub to: String,
    /// What to say, in full. The recipient sees who wrote it. Leave it out
    /// when you are posting `again`.
    pub text: Option<String>,
    /// Post the draft this room last handed back to you, word for word — your
    /// own bounced call still holds it, so there is nothing to write again.
    /// Only for a `#room` that bounced you, and never beside `text`.
    pub again: Option<bool>,
}

/// What a call says to write: its own text, or the latest draft this room
/// bounced (ADR-0053 §6). One or the other — a call that says both says two
/// different things, and a call that says neither says nothing at all.
fn said(text: Option<String>, again: Option<bool>) -> Result<Said, ToolError> {
        match (text, again.unwrap_or(false)) {
            (Some(_), true) => Err(refused("give `text` or `again`, not both")),
            (Some(text), false) => Ok(Said::Text(text)),
            (None, true) => Ok(Said::Again),
            (None, false) => Err(refused(
                "say what to write in `text`, or repeat what a room bounced with `again: true`",
            )),
        }
    }

/// An input the caller is asked to correct and call again with.
fn refused(said: &str) -> ToolError {
    ToolError::InvalidInput(said.into())
}

/// What an `again` addressed to an agent is told: only a room bounces, so
/// only a room holds a draft to repeat.
const NOT_A_ROOM: &str = "`again` repeats a bounced room post; write to an agent with `text`";

/// Posting into another session's queue: this session's own transcript is
/// unchanged by it, and the target gates whatever it then does.
#[derive(Debug, Default, Clone, Copy)]
pub struct MessageTool;

#[async_trait]
impl Tool for MessageTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: SEND_MESSAGE.into(),
            description: DESCRIPTION.into(),
            input_schema: input_schema::<MessageArgs>(),
            meta: Default::default(),
        }
    }

    fn traits(&self, _input: &Value) -> ToolTraits {
        crate::traits()
    }

    fn subjects(&self, input: &Value, _cwd: &Path) -> Vec<Subject> {
        serde_json::from_value::<MessageArgs>(input.clone())
            .map(|args| vec![Subject::Name { name: args.to }])
            .unwrap_or_default()
    }

    async fn call(&self, input: Value, cx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let args: MessageArgs =
            serde_json::from_value(input).map_err(|e| ToolError::InvalidInput(e.to_string()))?;
        let said = said(args.text, args.again)?;
        let to = match names::resolve(&cx.host, &cx.session, &args.to).await {
            Ok(to) => to,
            Err(e) => return Ok(ToolOutput::error(e.message)),
        };
        let from = names::speaker(&cx.host, &cx.session).await;
        match (to.driver, said) {
            (Driver::Log, said) => {
                let Some(writer) = room_post::find(&cx.host) else {
                    return Ok(ToolOutput::error("rooms are unavailable: no room post service"));
                };
                let result = writer.post(Request {
                    room: to.id.clone(), caller: cx.session.clone(), author: from.clone(), said, cut: cx.item.clone(),
                }).await.map_err(|error| ToolError::Failed(error.to_string()))?;
                if let Posted::Bounced(text) = result {
                    return Ok(ToolOutput::error(text));
                }
            }
            (Driver::Model, Said::Text(text)) => {
                let input = Input::text(text, origin(Some(from.clone())));
                cx.host.deliver(&to.id, IntentId::mint(), input, Delivery::Wake)
                    .await.map_err(|error| ToolError::Failed(error.message))?;
            }
            (Driver::Model, Said::Again) => return Err(refused(NOT_A_ROOM)),
        }
        let addressed = args.to.trim();
        let mut out = ToolOutput::text(receipt(addressed, to.driver));
        out.display = Some(card(addressed, &from, to.driver));
        Ok(out)
    }
}

