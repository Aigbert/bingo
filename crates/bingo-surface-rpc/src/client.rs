//! The JSON-RPC reference client, used by the server contract tests.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;
use bingo_jsonrpc::{Handler, Peer, PeerError};
use bingo_sdk::{
    Activation, Answer, Attachment, Catalog, CatalogKind, ClientIdentity, CloseReason, Delivery,
    ErrorCode, Frame, FrameStream, GatewayEvent, GatewayStream, HistoryChunk, HistoryPage, HostApi,
    Input, IntentId, InteractionId, InterruptScope, KernelError, OpenOptions, Seq, SessionFilter,
    SessionHandle, SessionId, SessionPort, SessionSelector, SessionSummary,
};
use futures::{FutureExt, Stream};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::codec::{self, Message, Notification, Outcome};
use crate::methods::{
    AnswerParams, ApproveResult, CatalogParams, DeliverParams, Empty, EventParams, EventsParams,
    ExtendParams, HistoryParams, InitializeParams, InitializeResult, InterruptParams, ListParams,
    ListResult, OpenParams, OpenResult, PROTOCOL, SessionParams, SignalParams, SubmitParams, name,
};

/// Where a session's frames go. The receiver waits here until `open` claims it,
/// so a frame that arrives before the caller knows the session id is kept.
struct Route {
    events: UnboundedSender<Frame>,
    waiting: Option<UnboundedReceiver<Frame>>,
}

impl Route {
    fn new() -> (Route, UnboundedReceiver<Frame>) {
        let (events, receiver) = unbounded_channel();
        (
            Route {
                events,
                waiting: None,
            },
            receiver,
        )
    }

    /// A route for frames nobody has asked for yet.
    fn pending() -> Route {
        let (mut route, receiver) = Route::new();
        route.waiting = Some(receiver);
        route
    }
}

/// Where a reply and a notification go.
#[derive(Default)]
struct Router {
    routes: Mutex<HashMap<SessionId, Route>>,
    gateway: Mutex<Option<UnboundedSender<GatewayEvent>>>,
}

impl Router {
    fn routes(&self) -> MutexGuard<'_, HashMap<SessionId, Route>> {
        self.routes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn notification(&self, notification: Notification) {
        match notification.method.as_str() {
            name::EVENT => match serde_json::from_value::<EventParams>(notification.params) {
                Ok(params) => self.frame(params),
                Err(error) => tracing::warn!(%error, "an event that is not a frame"),
            },
            name::GATEWAY_EVENT => {
                match serde_json::from_value::<GatewayEvent>(notification.params) {
                    Ok(event) => self.gateway_event(event),
                    Err(error) => tracing::warn!(%error, "a gateway event that is not one"),
                }
            }
            other => tracing::debug!(other, "an unknown notification"),
        }
    }

    fn frame(&self, params: EventParams) {
        let mut routes = self.routes();
        let route = routes
            .entry(params.route().clone())
            .or_insert_with(Route::pending);
        if route.events.send(params.frame).is_err() {
            tracing::debug!("a frame for a stream the caller dropped");
        }
    }

    fn gateway_event(&self, event: GatewayEvent) {
        let gateway = self
            .gateway
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(events) = gateway.as_ref()
            && events.send(event).is_err()
        {
            tracing::debug!("a gateway event for a stream the caller dropped");
        }
    }

    /// The session's frames, including any that arrived before this call.
    fn claim(&self, session: &SessionId) -> FrameStream {
        let mut routes = self.routes();
        match routes
            .get_mut(session)
            .and_then(|route| route.waiting.take())
        {
            Some(waiting) => stream(waiting),
            None => stream(install(&mut routes, session)),
        }
    }

    /// A fresh stream that takes over the routing; the previous one ends.
    fn resubscribe(&self, session: &SessionId) -> FrameStream {
        let mut routes = self.routes();
        stream(install(&mut routes, session))
    }

    fn forget(&self, session: &SessionId) {
        self.routes().remove(session);
    }

    fn subscribe_gateway(&self) -> GatewayStream {
        let (events, receiver) = unbounded_channel();
        *self
            .gateway
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(events);
        stream(receiver)
    }

    /// The connection ended; everyone waiting learns it rather than hanging.
    fn disconnect(&self) {
        self.routes().clear();
    }
}

fn install(
    routes: &mut HashMap<SessionId, Route>,
    session: &SessionId,
) -> UnboundedReceiver<Frame> {
    let (route, receiver) = Route::new();
    routes.insert(session.clone(), route);
    receiver
}

/// Unbounded on purpose: the reader task must never block on a slow consumer,
/// or a reply would queue behind the frames of the call that is awaiting it.
fn stream<T: Send + 'static>(
    mut receiver: UnboundedReceiver<T>,
) -> Pin<Box<dyn Stream<Item = T> + Send>> {
    Box::pin(futures::stream::poll_fn(move |context| {
        receiver.poll_recv(context)
    }))
}

impl Handler for Router {
    fn handle(
        &self,
        message: Message,
        _: bingo_jsonrpc::Sender,
    ) -> futures::future::BoxFuture<'_, ()> {
        async move {
            match message {
                Message::Notification(note) => self.notification(note),
                Message::Response(response) => {
                    if let Outcome::Error(error) = response.outcome {
                        tracing::error!(error = %error.message, "a request failed");
                    }
                }
                Message::Request(request) => {
                    tracing::warn!(method = %request.method, "the server asked, which it may not")
                }
            }
        }
        .boxed()
    }
    fn closed(&self, _: &PeerError) {
        self.disconnect();
    }
}

struct Connection {
    peer: Peer,
    router: Arc<Router>,
}

fn transport(error: PeerError) -> KernelError {
    match error {
        PeerError::Remote(error) => codec::to_kernel(error),
        error => KernelError::new(ErrorCode::Offline, error.to_string()),
    }
}

impl Connection {
    async fn call<P: Serialize, R: DeserializeOwned>(
        &self,
        method: &'static str,
        params: &P,
    ) -> Result<R, KernelError> {
        let value = self
            .peer
            .request(method, parameters(params)?)
            .await
            .map_err(transport)?;
        serde_json::from_value(value).map_err(|error| {
            KernelError::new(
                ErrorCode::Internal,
                format!("{method} answered with something unreadable: {error}"),
            )
        })
    }

    fn fire<P: Serialize>(&self, method: &'static str, params: &P) {
        if let Err(error) =
            parameters(params).and_then(|params| self.peer.fire(method, params).map_err(transport))
        {
            tracing::error!(%error, method, "a write never reached the server");
        }
    }
}

fn parameters(params: &impl Serialize) -> Result<Value, KernelError> {
    serde_json::to_value(params).map_err(|error| {
        KernelError::new(ErrorCode::InvalidInput, format!("unserialisable: {error}"))
    })
}

/// A kernel on the other end of a pipe.
#[derive(Clone)]
pub struct RemoteKernel {
    connection: Arc<Connection>,
}

impl RemoteKernel {
    pub fn connect<R, W>(reader: R, writer: W) -> RemoteKernel
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let router = Arc::new(Router::default());
        let peer = Peer::spawn(reader, writer, router.clone());
        RemoteKernel {
            connection: Arc::new(Connection { peer, router }),
        }
    }

    /// The handshake. Every later `open` carries this identity, so `HostApi`'s
    /// `who` argument is ignored by this implementation (ADR-0007 puts no
    /// identity on `session/open`).
    pub async fn initialize(
        &self,
        client: ClientIdentity,
    ) -> Result<InitializeResult, KernelError> {
        self.connection
            .call(
                name::INITIALIZE,
                &InitializeParams {
                    client,
                    protocol: PROTOCOL,
                },
            )
            .await
    }

    pub async fn shutdown(&self) -> Result<(), KernelError> {
        let Empty {} = self.connection.call(name::SHUTDOWN, &Empty {}).await?;
        Ok(())
    }
}

#[async_trait]
impl HostApi for RemoteKernel {
    async fn sessions(&self, filter: SessionFilter) -> Result<Vec<SessionSummary>, KernelError> {
        let result: ListResult = self
            .connection
            .call(name::SESSION_LIST, &ListParams { filter })
            .await?;
        Ok(result.sessions)
    }

    async fn open(
        &self,
        selector: SessionSelector,
        _who: ClientIdentity,
        options: OpenOptions,
    ) -> Result<Attachment, KernelError> {
        if options.max_snapshot_bytes.is_some() || options.tree_backfill.is_some() {
            return Err(KernelError::new(
                ErrorCode::InvalidInput,
                "RemoteKernel cannot represent incomplete event references; use a ref-aware RPC client",
            ));
        }
        let OpenResult {
            session,
            snapshot,
            history,
            ..
        }: OpenResult = self
            .connection
            .call(name::SESSION_OPEN, &OpenParams { selector, options })
            .await?;
        let events = self.connection.router.claim(&session);
        let handle = RemoteSession {
            connection: Arc::clone(&self.connection),
            session: session.clone(),
        };
        Ok(Attachment {
            session,
            snapshot,
            history,
            events,
            handle: SessionHandle(Arc::new(handle)),
        })
    }

    /// The wire's close is always a client detaching, so `reason` stays here.
    async fn close(&self, session: &SessionId, _reason: CloseReason) -> Result<(), KernelError> {
        let params = SessionParams {
            session: session.clone(),
        };
        let Empty {} = self.connection.call(name::SESSION_CLOSE, &params).await?;
        self.connection.router.forget(session);
        Ok(())
    }

    async fn fork(
        &self,
        session: &SessionId,
        at: Option<bingo_sdk::TurnId>,
    ) -> Result<SessionId, KernelError> {
        let result: crate::methods::ForkResult = self
            .connection
            .call(
                name::SESSION_FORK,
                &crate::methods::ForkParams {
                    session: session.clone(),
                    at,
                },
            )
            .await?;
        Ok(result.session)
    }
    async fn delete(&self, session: &SessionId) -> Result<(), KernelError> {
        let params = SessionParams {
            session: session.clone(),
        };
        let Empty {} = self.connection.call(name::SESSION_DELETE, &params).await?;
        self.connection.router.forget(session);
        Ok(())
    }

    async fn deliver(
        &self,
        to: &SessionId,
        intent: IntentId,
        input: Input,
        delivery: Delivery,
    ) -> Result<(), KernelError> {
        let params = DeliverParams {
            session: to.clone(),
            intent,
            input,
            delivery,
        };
        let Empty {} = self.connection.call(name::SESSION_DELIVER, &params).await?;
        Ok(())
    }

    async fn extend(
        &self,
        session: &SessionId,
        plugin: &str,
        kind: &str,
        payload: Value,
    ) -> Result<(), KernelError> {
        let params = ExtendParams {
            session: session.clone(),
            plugin: plugin.to_string(),
            kind: kind.to_string(),
            payload,
        };
        let Empty {} = self.connection.call(name::SESSION_EXTEND, &params).await?;
        Ok(())
    }

    async fn render(
        &self,
        session: &SessionId,
        request: bingo_sdk::render::RenderRequest,
    ) -> Result<bingo_sdk::View, KernelError> {
        self.connection
            .call(
                name::UI_RENDER,
                &crate::methods::RenderParams {
                    session: session.clone(),
                    request,
                },
            )
            .await
    }

    async fn signal(
        &self,
        session: &SessionId,
        plugin: &str,
        kind: &str,
        payload: Value,
    ) -> Result<(), KernelError> {
        let params = SignalParams {
            session: session.clone(),
            plugin: plugin.to_string(),
            kind: kind.to_string(),
            payload,
        };
        let Empty {} = self.connection.call(name::SESSION_SIGNAL, &params).await?;
        Ok(())
    }

    async fn approve(&self, request: bingo_sdk::Approval) -> Result<String, KernelError> {
        let result: ApproveResult = self.connection.call(name::APPROVE, &request).await?;
        Ok(result.digest)
    }

    async fn catalog(&self, kind: CatalogKind) -> Result<Catalog, KernelError> {
        self.connection
            .call(
                name::CATALOG_READ,
                &CatalogParams {
                    kind,
                    session: None,
                },
            )
            .await
    }

    async fn catalog_for(
        &self,
        session: &SessionId,
        kind: CatalogKind,
    ) -> Result<Catalog, KernelError> {
        self.connection
            .call(
                name::CATALOG_READ,
                &CatalogParams {
                    kind,
                    session: Some(session.clone()),
                },
            )
            .await
    }

    fn gateway_events(&self) -> GatewayStream {
        let events = self.connection.router.subscribe_gateway();
        self.connection.fire(name::GATEWAY_SUBSCRIBE, &Empty {});
        events
    }

    fn service_any(&self, _key: &str) -> Option<Arc<dyn std::any::Any + Send + Sync>> {
        None
    }
}

/// One session's mailbox, on the far side of the pipe.
pub struct RemoteSession {
    connection: Arc<Connection>,
    session: SessionId,
}

#[async_trait]
impl SessionPort for RemoteSession {
    fn submit(&self, intent: IntentId, input: Input) {
        self.connection.fire(
            name::SESSION_SUBMIT,
            &SubmitParams {
                session: self.session.clone(),
                intent,
                input,
            },
        );
    }

    fn interrupt(&self, intent: IntentId, scope: InterruptScope) {
        self.connection.fire(
            name::SESSION_INTERRUPT,
            &InterruptParams {
                session: self.session.clone(),
                intent,
                scope,
            },
        );
    }

    fn answer(
        &self,
        intent: IntentId,
        interaction: InteractionId,
        answer: Answer,
        activation: Activation,
    ) {
        self.connection.fire(
            name::SESSION_ANSWER,
            &AnswerParams {
                session: self.session.clone(),
                intent,
                interaction,
                answer,
                activation,
            },
        );
    }

    async fn history(&self, page: HistoryPage) -> Result<HistoryChunk, KernelError> {
        if page.max_bytes.is_some() {
            return Err(KernelError::new(
                ErrorCode::InvalidInput,
                "RemoteKernel cannot represent incomplete history; use a ref-aware RPC client",
            ));
        }
        self.connection
            .call(
                name::SESSION_HISTORY,
                &HistoryParams {
                    session: self.session.clone(),
                    page,
                },
            )
            .await
    }

    async fn events_since(&self, since: Seq) -> Result<FrameStream, KernelError> {
        // Installed before the request, so a frame arriving between here and the
        // server's swap reaches the new stream instead of the one being dropped.
        let events = self.connection.router.resubscribe(&self.session);
        let params = EventsParams {
            session: self.session.clone(),
            since,
        };
        let Empty {} = self.connection.call(name::SESSION_EVENTS, &params).await?;
        Ok(events)
    }
}
