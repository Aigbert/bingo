use crate::*;
use serde_json::Value;
use std::sync::Arc;

/// Every HostApi verb, with the arguments supplied by its caller.
#[derive(Clone)]
pub enum HostCall {
    Render {
        session: SessionId,
        request: crate::render::RenderRequest,
    },
    Complete {
        session: SessionId,
        turn: Option<TurnId>,
        request: Completion,
    },
    Sessions {
        filter: SessionFilter,
    },
    Open {
        selector: SessionSelector,
        who: ClientIdentity,
        options: OpenOptions,
    },
    Close {
        session: SessionId,
        reason: CloseReason,
    },
    Delete {
        session: SessionId,
    },
    Deliver {
        to: SessionId,
        intent: IntentId,
        input: Input,
        delivery: Delivery,
    },
    Extend {
        session: SessionId,
        plugin: String,
        kind: String,
        payload: Value,
    },
    Signal {
        session: SessionId,
        plugin: String,
        kind: String,
        payload: Value,
    },
    Invoke {
        session: SessionId,
        call: ToolCall,
    },
    Ask {
        session: SessionId,
        kind: InteractionKind,
        answers: Vec<AnswerSpec>,
    },
    Rewind {
        session: SessionId,
        to_turn: TurnId,
    },
    Withdraw {
        session: SessionId,
        intent: IntentId,
        who: ClientIdentity,
    },
    Reconfigure {
        session: SessionId,
        change: SessionChange,
    },
    Catalog {
        kind: CatalogKind,
    },
    Notice {
        level: Level,
        code: String,
        text: String,
    },
    GatewayEvents,
    ServiceAny {
        key: String,
    },
    ServiceWire {
        key: String,
    },
    OpenService {
        key: String,
        wire: Arc<dyn WireService>,
    },
}

impl std::fmt::Debug for HostCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Render { session, request } => {
                fields(f, "Render", &[("session", session), ("request", request)])
            }
            Self::Complete {
                session,
                turn,
                request,
            } => fields(
                f,
                "Complete",
                &[("session", session), ("turn", turn), ("request", request)],
            ),
            Self::Sessions { filter } => {
                f.debug_struct("Sessions").field("filter", filter).finish()
            }
            Self::Open {
                selector,
                who,
                options,
            } => fields(
                f,
                "Open",
                &[("selector", selector), ("who", who), ("options", options)],
            ),
            Self::Close { session, reason } => f
                .debug_struct("Close")
                .field("session", session)
                .field("reason", reason)
                .finish(),
            Self::Delete { session } => f.debug_struct("Delete").field("session", session).finish(),
            Self::Deliver {
                to,
                intent,
                input,
                delivery,
            } => f
                .debug_struct("Deliver")
                .field("to", to)
                .field("intent", intent)
                .field("input", input)
                .field("delivery", delivery)
                .finish(),
            Self::Extend {
                session,
                plugin,
                kind,
                payload,
            } => f
                .debug_struct("Extend")
                .field("session", session)
                .field("plugin", plugin)
                .field("kind", kind)
                .field("payload", payload)
                .finish(),
            Self::Signal {
                session,
                plugin,
                kind,
                payload,
            } => f
                .debug_struct("Signal")
                .field("session", session)
                .field("plugin", plugin)
                .field("kind", kind)
                .field("payload", payload)
                .finish(),
            Self::Invoke { session, call } => f
                .debug_struct("Invoke")
                .field("session", session)
                .field("call", call)
                .finish(),
            Self::Ask {
                session,
                kind,
                answers,
            } => f
                .debug_struct("Ask")
                .field("session", session)
                .field("kind", kind)
                .field("answers", answers)
                .finish(),
            Self::Rewind { session, to_turn } => f
                .debug_struct("Rewind")
                .field("session", session)
                .field("to_turn", to_turn)
                .finish(),
            Self::Withdraw {
                session,
                intent,
                who,
            } => f
                .debug_struct("Withdraw")
                .field("session", session)
                .field("intent", intent)
                .field("who", who)
                .finish(),
            Self::Reconfigure { session, change } => f
                .debug_struct("Reconfigure")
                .field("session", session)
                .field("change", change)
                .finish(),
            Self::Catalog { kind } => f.debug_struct("Catalog").field("kind", kind).finish(),
            Self::Notice { level, code, text } => f
                .debug_struct("Notice")
                .field("level", level)
                .field("code", code)
                .field("text", text)
                .finish(),
            Self::GatewayEvents => f.write_str("GatewayEvents"),
            Self::ServiceAny { key } => f.debug_struct("ServiceAny").field("key", key).finish(),
            Self::ServiceWire { key } => f.debug_struct("ServiceWire").field("key", key).finish(),
            Self::OpenService { key, wire: _ } => f
                .debug_struct("OpenService")
                .field("key", key)
                .field("wire", &"WireService")
                .finish(),
        }
    }
}

fn fields(
    f: &mut std::fmt::Formatter<'_>,
    name: &str,
    fields: &[(&str, &dyn std::fmt::Debug)],
) -> std::fmt::Result {
    let mut value = f.debug_struct(name);
    for (key, field) in fields {
        value.field(key, field);
    }
    value.finish()
}
