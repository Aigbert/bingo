use super::host::{TestHost, locked};
use super::{HostCall, NoHost};
use crate::*;
use async_trait::async_trait;
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

#[async_trait]
impl HostApi for TestHost {
    async fn sessions(&self, filter: SessionFilter) -> Result<Vec<SessionSummary>, KernelError> {
        self.record(HostCall::Sessions {
            filter: filter.clone(),
        });
        let script = locked(&self.inner.scripts).sessions.clone();
        if let Some(script) = script {
            return script((filter,)).await;
        }
        if let Some(table) = locked(&self.inner.sessions).as_mut() {
            return table.sessions(filter);
        }
        NoHost.sessions(filter).await
    }

    async fn open(
        &self,
        selector: SessionSelector,
        who: ClientIdentity,
        options: OpenOptions,
    ) -> Result<Attachment, KernelError> {
        self.record(HostCall::Open {
            selector: selector.clone(),
            who: who.clone(),
            options,
        });
        let script = locked(&self.inner.scripts).open.clone();
        if let Some(script) = script {
            return script((selector, who, options)).await;
        }
        if let Some(table) = locked(&self.inner.sessions).as_mut() {
            return table.open(selector);
        }
        NoHost.open(selector, who, options).await
    }

    async fn close(&self, session: &SessionId, reason: CloseReason) -> Result<(), KernelError> {
        self.record(HostCall::Close {
            session: session.clone(),
            reason: reason.clone(),
        });
        let script = locked(&self.inner.scripts).close.clone();
        if let Some(script) = script {
            return script((session.clone(), reason)).await;
        }
        if let Some(table) = locked(&self.inner.sessions).as_mut() {
            return table.close(session, reason);
        }
        NoHost.close(session, reason).await
    }

    async fn delete(&self, session: &SessionId) -> Result<(), KernelError> {
        self.record(HostCall::Delete {
            session: session.clone(),
        });
        let script = locked(&self.inner.scripts).delete.clone();
        if let Some(script) = script {
            return script((session.clone(),)).await;
        }
        if let Some(table) = locked(&self.inner.sessions).as_mut() {
            return table.delete(session);
        }
        NoHost.delete(session).await
    }

    async fn deliver(
        &self,
        to: &SessionId,
        intent: IntentId,
        input: Input,
        delivery: Delivery,
    ) -> Result<(), KernelError> {
        self.record(HostCall::Deliver {
            to: to.clone(),
            intent: intent.clone(),
            input: input.clone(),
            delivery,
        });
        let script = locked(&self.inner.scripts).deliver.clone();
        if let Some(script) = script {
            return script((to.clone(), intent, input, delivery)).await;
        }
        NoHost.deliver(to, intent, input, delivery).await
    }

    async fn extend(
        &self,
        session: &SessionId,
        plugin: &str,
        kind: &str,
        payload: Value,
    ) -> Result<(), KernelError> {
        self.record(HostCall::Extend {
            session: session.clone(),
            plugin: plugin.to_string(),
            kind: kind.to_string(),
            payload: payload.clone(),
        });
        let script = locked(&self.inner.scripts).extend.clone();
        if let Some(script) = script {
            return script((
                session.clone(),
                plugin.to_string(),
                kind.to_string(),
                payload,
            ))
            .await;
        }
        NoHost.extend(session, plugin, kind, payload).await
    }

    async fn signal(
        &self,
        session: &SessionId,
        plugin: &str,
        kind: &str,
        payload: Value,
    ) -> Result<(), KernelError> {
        self.record(HostCall::Signal {
            session: session.clone(),
            plugin: plugin.to_string(),
            kind: kind.to_string(),
            payload: payload.clone(),
        });
        let script = locked(&self.inner.scripts).signal.clone();
        if let Some(script) = script {
            return script((
                session.clone(),
                plugin.to_string(),
                kind.to_string(),
                payload,
            ))
            .await;
        }
        NoHost.signal(session, plugin, kind, payload).await
    }

    async fn invoke(
        &self,
        session: &SessionId,
        call: ToolCall,
    ) -> Result<ToolOutcome, KernelError> {
        self.record(HostCall::Invoke {
            session: session.clone(),
            call: call.clone(),
        });
        let script = locked(&self.inner.scripts).invoke.clone();
        if let Some(script) = script {
            return script((session.clone(), call)).await;
        }
        NoHost.invoke(session, call).await
    }

    async fn ask(
        &self,
        session: &SessionId,
        kind: InteractionKind,
        answers: Vec<AnswerSpec>,
    ) -> Result<Answer, KernelError> {
        self.record(HostCall::Ask {
            session: session.clone(),
            kind: kind.clone(),
            answers: answers.clone(),
        });
        let script = locked(&self.inner.scripts).ask.clone();
        if let Some(script) = script {
            return script((session.clone(), kind, answers)).await;
        }
        NoHost.ask(session, kind, answers).await
    }

    async fn rewind(&self, session: &SessionId, to_turn: &TurnId) -> Result<u32, KernelError> {
        self.record(HostCall::Rewind {
            session: session.clone(),
            to_turn: to_turn.clone(),
        });
        let script = locked(&self.inner.scripts).rewind.clone();
        if let Some(script) = script {
            return script((session.clone(), to_turn.clone())).await;
        }
        NoHost.rewind(session, to_turn).await
    }

    async fn withdraw(
        &self,
        session: &SessionId,
        intent: &IntentId,
        who: ClientIdentity,
    ) -> Result<Input, KernelError> {
        self.record(HostCall::Withdraw {
            session: session.clone(),
            intent: intent.clone(),
            who: who.clone(),
        });
        let script = locked(&self.inner.scripts).withdraw.clone();
        if let Some(script) = script {
            return script((session.clone(), intent.clone(), who)).await;
        }
        NoHost.withdraw(session, intent, who).await
    }

    async fn reconfigure(
        &self,
        session: &SessionId,
        change: SessionChange,
    ) -> Result<(), KernelError> {
        self.record(HostCall::Reconfigure {
            session: session.clone(),
            change: change.clone(),
        });
        let script = locked(&self.inner.scripts).reconfigure.clone();
        if let Some(script) = script {
            return script((session.clone(), change)).await;
        }
        NoHost.reconfigure(session, change).await
    }

    async fn catalog(&self, kind: CatalogKind) -> Result<Catalog, KernelError> {
        self.record(HostCall::Catalog { kind });
        let script = locked(&self.inner.scripts).catalog.clone();
        if let Some(script) = script {
            return script((kind,)).await;
        }
        NoHost.catalog(kind).await
    }

    async fn notice(&self, level: Level, code: &str, text: &str) -> Result<(), KernelError> {
        self.record(HostCall::Notice {
            level,
            code: code.to_string(),
            text: text.to_string(),
        });
        let script = locked(&self.inner.scripts).notice.clone();
        if let Some(script) = script {
            return script((level, code.to_string(), text.to_string())).await;
        }
        NoHost.notice(level, code, text).await
    }

    fn gateway_events(&self) -> GatewayStream {
        self.record(HostCall::GatewayEvents);
        let script = locked(&self.inner.immediate).gateway_events.clone();
        if let Some(script) = script {
            return script(());
        }
        match locked(&self.inner.gateway).take() {
            Some(events) => Box::pin(events),
            None => NoHost.gateway_events(),
        }
    }

    fn service_any(&self, key: &str) -> Option<Arc<dyn Any + Send + Sync>> {
        self.record(HostCall::ServiceAny {
            key: key.to_string(),
        });
        let script = locked(&self.inner.immediate).service_any.clone();
        if let Some(script) = script {
            return script((key.to_string(),));
        }
        self.inner.services.value(key)
    }

    fn service_wire(&self, key: &str) -> Option<Arc<dyn WireService>> {
        self.record(HostCall::ServiceWire {
            key: key.to_string(),
        });
        let script = locked(&self.inner.immediate).service_wire.clone();
        if let Some(script) = script {
            return script((key.to_string(),));
        }
        self.inner.services.wire(key)
    }

    fn open_service(&self, key: &str, wire: Arc<dyn WireService>) -> Result<(), KernelError> {
        self.record(HostCall::OpenService {
            key: key.to_string(),
            wire: wire.clone(),
        });
        let script = locked(&self.inner.immediate).open_service.clone();
        if let Some(script) = script {
            return script((key.to_string(), wire));
        }
        self.inner
            .services
            .open(key, wire)
            .map_err(|why| KernelError::new(ErrorCode::InvalidInput, why))
    }
}
