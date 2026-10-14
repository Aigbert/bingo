use super::host::{TestHost, locked};
use crate::*;
use futures::future::BoxFuture;
use serde_json::Value;
use std::any::Any;
use std::future::Future;
use std::sync::Arc;

type Script<A, R> = Arc<dyn Fn(A) -> BoxFuture<'static, Result<R, KernelError>> + Send + Sync>;
type Immediate<A, R> = Arc<dyn Fn(A) -> R + Send + Sync>;

type QuestionScript = Script<(SessionId, InteractionKind, Vec<AnswerSpec>), Answer>;

type ServiceLookup = Immediate<(String,), Option<Arc<dyn Any + Send + Sync>>>;

type WireLookup = Immediate<(String,), Option<Arc<dyn WireService>>>;

type ServiceOpening = Immediate<(String, Arc<dyn WireService>), Result<(), KernelError>>;

type CompletionScript = Script<(SessionId, Option<TurnId>, Completion), Vec<ModelEvent>>;

#[derive(Default)]
pub(super) struct Scripts {
    pub complete: Option<CompletionScript>,
    pub render: Option<Script<(SessionId, crate::render::RenderRequest), View>>,
    pub sessions: Option<Script<(SessionFilter,), Vec<SessionSummary>>>,
    pub open: Option<Script<(SessionSelector, ClientIdentity, OpenOptions), Attachment>>,
    pub close: Option<Script<(SessionId, CloseReason), ()>>,
    pub delete: Option<Script<(SessionId,), ()>>,
    pub deliver: Option<Script<(SessionId, IntentId, Input, Delivery), ()>>,
    pub extend: Option<Script<(SessionId, String, String, Value), ()>>,
    pub signal: Option<Script<(SessionId, String, String, Value), ()>>,
    pub invoke: Option<Script<(SessionId, ToolCall), ToolOutcome>>,
    pub ask: Option<QuestionScript>,
    pub rewind: Option<Script<(SessionId, TurnId), u32>>,
    pub withdraw: Option<Script<(SessionId, IntentId, ClientIdentity), Input>>,
    pub reconfigure: Option<Script<(SessionId, SessionChange), ()>>,
    pub catalog: Option<Script<(CatalogKind,), Catalog>>,
    pub notice: Option<Script<(Level, String, String), ()>>,
}

#[derive(Default)]
pub(super) struct ImmediateScripts {
    pub gateway_events: Option<Immediate<(), GatewayStream>>,
    pub service_any: Option<ServiceLookup>,
    pub service_wire: Option<WireLookup>,
    pub open_service: Option<ServiceOpening>,
}

impl TestHost {
    pub fn on_render<F, Fut>(&self, script: F)
    where
        F: Fn(SessionId, crate::render::RenderRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<View, KernelError>> + Send + 'static,
    {
        locked(&self.inner.scripts).render = Some(Arc::new(move |(session, request)| {
            Box::pin(script(session, request))
        }));
    }

    /// Override `complete`; the call is recorded before this script runs.
    pub fn on_complete<F, Fut>(&self, script: F)
    where
        F: Fn(SessionId, Option<TurnId>, Completion) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Vec<ModelEvent>, KernelError>> + Send + 'static,
    {
        locked(&self.inner.scripts).complete = Some(Arc::new(move |(session, turn, request)| {
            Box::pin(script(session, turn, request))
        }));
    }

    /// Override `sessions`; the call is recorded before this script runs.
    pub fn on_sessions<F, Fut>(&self, script: F)
    where
        F: Fn(SessionFilter) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Vec<SessionSummary>, KernelError>> + Send + 'static,
    {
        locked(&self.inner.scripts).sessions =
            Some(Arc::new(move |(filter,)| Box::pin(script(filter))));
    }

    /// Override `open`; the call is recorded before this script runs.
    pub fn on_open<F, Fut>(&self, script: F)
    where
        F: Fn(SessionSelector, ClientIdentity, OpenOptions) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Attachment, KernelError>> + Send + 'static,
    {
        locked(&self.inner.scripts).open = Some(Arc::new(move |(selector, who, options)| {
            Box::pin(script(selector, who, options))
        }));
    }

    /// Override `close`; the call is recorded before this script runs.
    pub fn on_close<F, Fut>(&self, script: F)
    where
        F: Fn(SessionId, CloseReason) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), KernelError>> + Send + 'static,
    {
        locked(&self.inner.scripts).close = Some(Arc::new(move |(session, reason)| {
            Box::pin(script(session, reason))
        }));
    }

    /// Override `delete`; the call is recorded before this script runs.
    pub fn on_delete<F, Fut>(&self, script: F)
    where
        F: Fn(SessionId) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), KernelError>> + Send + 'static,
    {
        locked(&self.inner.scripts).delete =
            Some(Arc::new(move |(session,)| Box::pin(script(session))));
    }

    /// Override `deliver`; the call is recorded before this script runs.
    pub fn on_deliver<F, Fut>(&self, script: F)
    where
        F: Fn(SessionId, IntentId, Input, Delivery) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), KernelError>> + Send + 'static,
    {
        locked(&self.inner.scripts).deliver =
            Some(Arc::new(move |(to, intent, input, delivery)| {
                Box::pin(script(to, intent, input, delivery))
            }));
    }

    /// Override `extend`; the call is recorded before this script runs.
    pub fn on_extend<F, Fut>(&self, script: F)
    where
        F: Fn(SessionId, String, String, Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), KernelError>> + Send + 'static,
    {
        locked(&self.inner.scripts).extend =
            Some(Arc::new(move |(session, plugin, kind, payload)| {
                Box::pin(script(session, plugin, kind, payload))
            }));
    }

    /// Override `signal`; the call is recorded before this script runs.
    pub fn on_signal<F, Fut>(&self, script: F)
    where
        F: Fn(SessionId, String, String, Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), KernelError>> + Send + 'static,
    {
        locked(&self.inner.scripts).signal =
            Some(Arc::new(move |(session, plugin, kind, payload)| {
                Box::pin(script(session, plugin, kind, payload))
            }));
    }

    /// Override `invoke`; the call is recorded before this script runs.
    pub fn on_invoke<F, Fut>(&self, script: F)
    where
        F: Fn(SessionId, ToolCall) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<ToolOutcome, KernelError>> + Send + 'static,
    {
        locked(&self.inner.scripts).invoke = Some(Arc::new(move |(session, call)| {
            Box::pin(script(session, call))
        }));
    }

    /// Override `ask`; the call is recorded before this script runs.
    pub fn on_ask<F, Fut>(&self, script: F)
    where
        F: Fn(SessionId, InteractionKind, Vec<AnswerSpec>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Answer, KernelError>> + Send + 'static,
    {
        locked(&self.inner.scripts).ask = Some(Arc::new(move |(session, kind, answers)| {
            Box::pin(script(session, kind, answers))
        }));
    }

    /// Override `rewind`; the call is recorded before this script runs.
    pub fn on_rewind<F, Fut>(&self, script: F)
    where
        F: Fn(SessionId, TurnId) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<u32, KernelError>> + Send + 'static,
    {
        locked(&self.inner.scripts).rewind = Some(Arc::new(move |(session, to_turn)| {
            Box::pin(script(session, to_turn))
        }));
    }

    /// Override `withdraw`; the call is recorded before this script runs.
    pub fn on_withdraw<F, Fut>(&self, script: F)
    where
        F: Fn(SessionId, IntentId, ClientIdentity) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Input, KernelError>> + Send + 'static,
    {
        locked(&self.inner.scripts).withdraw = Some(Arc::new(move |(session, intent, who)| {
            Box::pin(script(session, intent, who))
        }));
    }

    /// Override `reconfigure`; the call is recorded before this script runs.
    pub fn on_reconfigure<F, Fut>(&self, script: F)
    where
        F: Fn(SessionId, SessionChange) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), KernelError>> + Send + 'static,
    {
        locked(&self.inner.scripts).reconfigure = Some(Arc::new(move |(session, change)| {
            Box::pin(script(session, change))
        }));
    }

    /// Override `catalog`; the call is recorded before this script runs.
    pub fn on_catalog<F, Fut>(&self, script: F)
    where
        F: Fn(CatalogKind) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Catalog, KernelError>> + Send + 'static,
    {
        locked(&self.inner.scripts).catalog = Some(Arc::new(move |(kind,)| Box::pin(script(kind))));
    }

    /// Override `notice`; the call is recorded before this script runs.
    pub fn on_notice<F, Fut>(&self, script: F)
    where
        F: Fn(Level, String, String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), KernelError>> + Send + 'static,
    {
        locked(&self.inner.scripts).notice = Some(Arc::new(move |(level, code, text)| {
            Box::pin(script(level, code, text))
        }));
    }

    /// Override `gateway_events`; the call is recorded before this script runs.
    pub fn on_gateway_events<F>(&self, script: F)
    where
        F: Fn() -> GatewayStream + Send + Sync + 'static,
    {
        locked(&self.inner.immediate).gateway_events = Some(Arc::new(move |()| script()));
    }

    /// Override `service_any`; the call is recorded before this script runs.
    pub fn on_service_any<F>(&self, script: F)
    where
        F: Fn(String) -> Option<Arc<dyn Any + Send + Sync>> + Send + Sync + 'static,
    {
        locked(&self.inner.immediate).service_any = Some(Arc::new(move |(key,)| script(key)));
    }

    /// Override `service_wire`; the call is recorded before this script runs.
    pub fn on_service_wire<F>(&self, script: F)
    where
        F: Fn(String) -> Option<Arc<dyn WireService>> + Send + Sync + 'static,
    {
        locked(&self.inner.immediate).service_wire = Some(Arc::new(move |(key,)| script(key)));
    }

    /// Override `open_service`; the call is recorded before this script runs.
    pub fn on_open_service<F>(&self, script: F)
    where
        F: Fn(String, Arc<dyn WireService>) -> Result<(), KernelError> + Send + Sync + 'static,
    {
        locked(&self.inner.immediate).open_service =
            Some(Arc::new(move |(key, wire)| script(key, wire)));
    }
}
