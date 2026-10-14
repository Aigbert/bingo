//! Plugin handles preserve provenance even when a client identity is forged.
use bingo_sdk::*;
use serde_json::Value;
use std::{any::Any, cell::Cell, sync::Arc};
use async_trait::async_trait;

thread_local! { static ANSWERING: Cell<bool> = const { Cell::new(false) }; }
pub(crate) fn answering() -> bool { ANSWERING.get() }

pub(crate) fn handle(host: HostHandle) -> HostHandle { HostHandle(Arc::new(PluginHost(host))) }
struct PluginHost(HostHandle);
#[async_trait]
impl HostApi for PluginHost {
    async fn render(
        &self,
        _session: &SessionId,
        request: bingo_sdk::render::RenderRequest,
    ) -> Result<bingo_sdk::View, KernelError> {
        self.0.render(_session, request).await
    }

    async fn invalidate(
        &self,
        _session: &SessionId,
        _requests: Vec<String>,
    ) -> Result<(), KernelError> {
        self.0.invalidate(_session, _requests).await
    }

    async fn complete(
        &self,
        _session: &SessionId,
        _turn: Option<&TurnId>,
        _request: Completion,
    ) -> Result<Vec<bingo_sdk::ModelEvent>, KernelError> {
        self.0.complete(_session, _turn, _request).await
    }

    async fn sessions(&self, filter: SessionFilter) -> Result<Vec<SessionSummary>, KernelError> {
        self.0.sessions(filter).await
    }

    async fn open(
        &self,
        selector: SessionSelector,
        who: ClientIdentity,
        options: OpenOptions,
    ) -> Result<Attachment, KernelError> {
        let mut attachment = self.0.open(selector, who, options).await?;
        attachment.handle = SessionHandle(Arc::new(PluginPort(attachment.handle)));
        Ok(attachment)
    }

    async fn close(&self, session: &SessionId, reason: CloseReason) -> Result<(), KernelError> {
        self.0.close(session, reason).await
    }

    async fn delete(&self, session: &SessionId) -> Result<(), KernelError> {
        self.0.delete(session).await
    }

    async fn deliver(
        &self,
        to: &SessionId,
        intent: IntentId,
        input: Input,
        delivery: Delivery,
    ) -> Result<(), KernelError> {
        self.0.deliver(to, intent, input, delivery).await
    }

    async fn extend(
        &self,
        session: &SessionId,
        plugin: &str,
        kind: &str,
        payload: Value,
    ) -> Result<(), KernelError> {
        self.0.extend(session, plugin, kind, payload).await
    }

    async fn signal(
        &self,
        session: &SessionId,
        plugin: &str,
        kind: &str,
        payload: Value,
    ) -> Result<(), KernelError> {
        self.0.signal(session, plugin, kind, payload).await
    }

    async fn invoke(
        &self,
        _session: &SessionId,
        _call: ToolCall,
    ) -> Result<ToolOutcome, KernelError> {
        self.0.invoke(_session, _call).await
    }

    async fn ask(
        &self,
        _session: &SessionId,
        _kind: InteractionKind,
        _answers: Vec<AnswerSpec>,
    ) -> Result<Answer, KernelError> {
        self.0.ask(_session, _kind, _answers).await
    }

    async fn rewind(&self, _session: &SessionId, _to_turn: &TurnId) -> Result<u32, KernelError> {
        self.0.rewind(_session, _to_turn).await
    }

    async fn withdraw(
        &self,
        _session: &SessionId,
        _intent: &IntentId,
        _who: ClientIdentity,
    ) -> Result<Input, KernelError> {
        self.0.withdraw(_session, _intent, _who).await
    }

    async fn reconfigure(
        &self,
        _session: &SessionId,
        _change: SessionChange,
    ) -> Result<(), KernelError> {
        self.0.reconfigure(_session, _change).await
    }

    async fn catalog(&self, kind: CatalogKind) -> Result<Catalog, KernelError> {
        self.0.catalog(kind).await
    }

    async fn notice(&self, _level: Level, _code: &str, _text: &str) -> Result<(), KernelError> {
        self.0.notice(_level, _code, _text).await
    }

    fn gateway_events(&self) -> GatewayStream {
        self.0.gateway_events()
    }

    fn service_any(&self, key: &str) -> Option<Arc<dyn Any + Send + Sync>> {
        self.0.service_any(key)
    }

    fn service_wire(&self, _key: &str) -> Option<Arc<dyn WireService>> {
        self.0.service_wire(_key)
    }

    fn open_service(&self, key: &str, _wire: Arc<dyn WireService>) -> Result<(), KernelError> {
        self.0.open_service(key, _wire)
    }
}

struct PluginPort(SessionHandle);
struct Answering(bool);
impl Drop for Answering { fn drop(&mut self) { ANSWERING.set(self.0); } }
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
