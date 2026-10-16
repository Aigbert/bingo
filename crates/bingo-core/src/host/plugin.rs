//! Plugin handles preserve provenance even when a client identity is forged.
use async_trait::async_trait;
use bingo_sdk::*;
use serde_json::Value;
use std::{
    any::Any,
    cell::Cell,
    sync::{Arc, Weak},
};

/// Start the enabled plugins in load order; each receives a host handle.
pub(super) async fn start(host: &super::Host) -> Result<(), super::HostError> {
    for plugin in &host.plugins {
        let manifest = plugin.manifest();
        if !host.registry.enabled(manifest.id) {
            continue;
        }
        if let Err(source) = plugin.start(host.plugin_host.clone()).await {
            if host
                .registry
                .switches
                .running
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .contains(manifest.id)
            {
                host.registry.unavailable(manifest.id, source.to_string());
                if let Err(error) = plugin.stop().await {
                    tracing::warn!(plugin = manifest.id, %error, "disabled plugin cleanup failed");
                }
                continue;
            }
            return Err(super::HostError::Start {
                plugin: manifest.id.into(),
                source,
            });
        }
    }
    Ok(())
}

thread_local! { static ANSWERING: Cell<bool> = const { Cell::new(false) }; }
pub(crate) fn answering() -> bool {
    ANSWERING.get()
}

pub(crate) fn handle(host: Weak<super::Host>) -> HostHandle {
    HostHandle(Arc::new(PluginHost(host)))
}
struct PluginHost(Weak<super::Host>);
impl PluginHost {
    fn inner(&self) -> HostHandle {
        match self.0.upgrade() {
            Some(host) => HostHandle(host),
            None => HostHandle(Arc::new(super::Unavailable)),
        }
    }
}
#[async_trait]
impl HostApi for PluginHost {
    async fn approve(&self, request: Approval) -> Result<String, KernelError> {
        self.inner().approve(request).await
    }

    async fn render(
        &self,
        _session: &SessionId,
        request: bingo_sdk::render::RenderRequest,
    ) -> Result<bingo_sdk::View, KernelError> {
        self.inner().render(_session, request).await
    }

    async fn invalidate(
        &self,
        _session: &SessionId,
        _requests: Vec<String>,
    ) -> Result<(), KernelError> {
        self.inner().invalidate(_session, _requests).await
    }

    async fn complete(
        &self,
        _session: &SessionId,
        _turn: Option<&TurnId>,
        _request: Completion,
    ) -> Result<Vec<bingo_sdk::ModelEvent>, KernelError> {
        self.inner().complete(_session, _turn, _request).await
    }

    async fn sessions(&self, filter: SessionFilter) -> Result<Vec<SessionSummary>, KernelError> {
        self.inner().sessions(filter).await
    }

    async fn open(
        &self,
        selector: SessionSelector,
        who: ClientIdentity,
        options: OpenOptions,
    ) -> Result<Attachment, KernelError> {
        let mut attachment = self.inner().open(selector, who, options).await?;
        attachment.handle = SessionHandle(Arc::new(PluginPort(attachment.handle)));
        Ok(attachment)
    }

    async fn close(&self, session: &SessionId, reason: CloseReason) -> Result<(), KernelError> {
        self.inner().close(session, reason).await
    }

    async fn read(&self, session: &SessionId) -> Result<SessionState, KernelError> {
        self.inner().read(session).await
    }

    async fn summary(&self, session: &SessionId) -> Result<SessionSummary, KernelError> {
        self.inner().summary(session).await
    }

    async fn check_delete(&self, session: &SessionId) -> Result<(), KernelError> {
        self.inner().check_delete(session).await
    }

    async fn open_elsewhere(&self, session: &SessionId) -> Result<bool, KernelError> {
        self.inner().open_elsewhere(session).await
    }

    async fn fork(
        &self,
        session: &SessionId,
        at: Option<TurnId>,
    ) -> Result<SessionId, KernelError> {
        self.inner().fork(session, at).await
    }

    async fn delete(&self, session: &SessionId) -> Result<(), KernelError> {
        self.inner().delete(session).await
    }

    async fn deliver(
        &self,
        to: &SessionId,
        intent: IntentId,
        input: Input,
        delivery: Delivery,
    ) -> Result<(), KernelError> {
        self.inner().deliver(to, intent, input, delivery).await
    }

    async fn extend(
        &self,
        session: &SessionId,
        plugin: &str,
        kind: &str,
        payload: Value,
    ) -> Result<(), KernelError> {
        self.inner().extend(session, plugin, kind, payload).await
    }

    async fn signal(
        &self,
        session: &SessionId,
        plugin: &str,
        kind: &str,
        payload: Value,
    ) -> Result<(), KernelError> {
        self.inner().signal(session, plugin, kind, payload).await
    }

    async fn transfer_signals(
        &self,
        session: &SessionId,
        plugin: &str,
        from: &ItemId,
        to: &ItemId,
    ) -> Result<(), KernelError> {
        self.inner()
            .transfer_signals(session, plugin, from, to)
            .await
    }

    async fn invoke(
        &self,
        _session: &SessionId,
        _call: ToolCall,
    ) -> Result<ToolOutcome, KernelError> {
        self.inner().invoke(_session, _call).await
    }

    async fn ask(
        &self,
        _session: &SessionId,
        _kind: InteractionKind,
        _answers: Vec<AnswerSpec>,
    ) -> Result<Answer, KernelError> {
        self.inner().ask(_session, _kind, _answers).await
    }

    async fn rewind(&self, _session: &SessionId, _to_turn: &TurnId) -> Result<u32, KernelError> {
        self.inner().rewind(_session, _to_turn).await
    }

    async fn withdraw(
        &self,
        _session: &SessionId,
        _intent: &IntentId,
        _who: ClientIdentity,
    ) -> Result<Input, KernelError> {
        self.inner().withdraw(_session, _intent, _who).await
    }

    async fn reconfigure(
        &self,
        _session: &SessionId,
        _change: SessionChange,
    ) -> Result<(), KernelError> {
        self.inner().reconfigure(_session, _change).await
    }

    fn catalog_changed(&self, kind: CatalogKind) {
        self.inner().catalog_changed(kind);
    }

    async fn catalog(&self, kind: CatalogKind) -> Result<Catalog, KernelError> {
        self.inner().catalog(kind).await
    }

    async fn catalog_for(
        &self,
        session: &SessionId,
        kind: CatalogKind,
    ) -> Result<Catalog, KernelError> {
        self.inner().catalog_for(session, kind).await
    }

    async fn notice(&self, _level: Level, _code: &str, _text: &str) -> Result<(), KernelError> {
        self.inner().notice(_level, _code, _text).await
    }

    fn gateway_events(&self) -> GatewayStream {
        self.inner().gateway_events()
    }

    fn service_any(&self, key: &str) -> Option<Arc<dyn Any + Send + Sync>> {
        self.inner().service_any(key)
    }

    fn service_wire(&self, _key: &str) -> Option<Arc<dyn WireService>> {
        self.inner().service_wire(_key)
    }

    fn open_service(&self, key: &str, _wire: Arc<dyn WireService>) -> Result<(), KernelError> {
        self.inner().open_service(key, _wire)
    }
}

struct PluginPort(SessionHandle);
struct Answering(bool);
impl Drop for Answering {
    fn drop(&mut self) {
        ANSWERING.set(self.0);
    }
}
#[async_trait]
impl SessionPort for PluginPort {
    fn submit(&self, intent: IntentId, input: Input) {
        self.0.submit(intent, input)
    }

    fn interrupt(&self, intent: IntentId, scope: InterruptScope) {
        self.0.interrupt(intent, scope)
    }

    fn answer(
        &self,
        intent: IntentId,
        interaction: InteractionId,
        answer: Answer,
        activation: Activation,
    ) {
        let _source = Answering(ANSWERING.replace(true));
        self.0.answer(intent, interaction, answer, activation);
    }

    async fn history(&self, page: HistoryPage) -> Result<HistoryChunk, KernelError> {
        self.0.history(page).await
    }

    async fn item_for_pin(&self, _id: &ItemId, _generation: u64) -> Result<Item, KernelError> {
        self.0.item_for_pin(_id, _generation).await
    }

    async fn events_since(&self, since: Seq) -> Result<FrameStream, KernelError> {
        self.0.events_since(since).await
    }
}

impl super::Host {
    pub(crate) fn effective_switches(
        &self,
        off: std::collections::BTreeSet<String>,
    ) -> std::collections::BTreeSet<String> {
        crate::plugins::effective_switches(&self.plugins, off)
    }
    pub(crate) fn plugins_changed(&self) {
        for kind in [CatalogKind::Plugins, CatalogKind::Commands] {
            self.catalog_changed(kind);
        }
    }
}
