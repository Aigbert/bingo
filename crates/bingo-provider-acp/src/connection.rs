//! ACP vocabulary over the shared JSON-RPC peer.
use crate::error::AcpError;
use crate::method::{self, Call, Incoming, Notify};
use agent_client_protocol_schema::v1::{
    CreateElicitationRequest, CreateElicitationResponse, Error as RpcError,
    RequestPermissionRequest, RequestPermissionResponse, SessionNotification,
};
use async_trait::async_trait;
use bingo_jsonrpc::codec::{Message, Response};
use bingo_jsonrpc::{Handler, Peer, PeerError};
use futures::FutureExt;
use serde_json::Value;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};

/// What the agent starts a line about. The connection answers nothing on its
/// own: it hands the question over and writes back what it is told.
#[async_trait]
pub trait Client: Send + Sync + 'static {
    /// A turn's stream. Notifications get no reply, so this cannot fail.
    async fn update(&self, notification: SessionNotification);

    /// The agent asking whether it may do something.
    async fn permission(
        &self,
        request: RequestPermissionRequest,
    ) -> Result<RequestPermissionResponse, RpcError>;

    /// The agent asking this client to collect something from a person.
    async fn elicitation(
        &self,
        request: CreateElicitationRequest,
    ) -> Result<CreateElicitationResponse, RpcError>;
}

pub struct Connection {
    peer: Peer,
}

impl Connection {
    pub fn spawn<R, W>(reader: R, writer: W, client: Arc<dyn Client>) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        Self {
            peer: Peer::spawn(reader, writer, Arc::new(Inbound(client))),
        }
    }

    pub fn is_alive(&self) -> bool {
        self.peer.is_alive()
    }

    pub(crate) fn mark_dead(&self) {
        self.peer.close();
    }

    pub async fn call<C: Call>(&self, params: C) -> Result<C::Response, AcpError> {
        self.call_seen(params).await.map(|(answer, _)| answer)
    }

    pub async fn call_seen<C: Call>(&self, params: C) -> Result<(C::Response, Value), AcpError> {
        let params = serde_json::to_value(params).map_err(AcpError::protocol)?;
        let answer = self
            .peer
            .request(C::METHOD, params)
            .await
            .map_err(transport)?;
        let parsed = serde_json::from_value(answer.clone())
            .map_err(|e| AcpError::protocol(format!("{}: {e}", C::METHOD)))?;
        Ok((parsed, answer))
    }

    pub fn notify<N: Notify>(&self, params: N) -> Result<(), AcpError> {
        let params = serde_json::to_value(params).map_err(AcpError::protocol)?;
        self.peer.notify(N::METHOD, params).map_err(transport)
    }
}

fn transport(error: PeerError) -> AcpError {
    match error {
        PeerError::Remote(error) => {
            let Ok(code) = i32::try_from(error.code) else {
                return AcpError::protocol("ACP error code is outside i32");
            };
            let mut refusal = RpcError::new(code, error.message);
            refusal.data = error.data;
            AcpError::Refused(refusal)
        }
        error => AcpError::transport(error),
    }
}

struct Inbound(Arc<dyn Client>);

impl Handler for Inbound {
    fn handle(
        &self,
        message: Message,
        sender: bingo_jsonrpc::Sender,
    ) -> futures::future::BoxFuture<'_, ()> {
        async move {
            match message {
                Message::Notification(note) => observe(&note.method, note.params, &self.0).await,
                Message::Request(request) => {
                    let response =
                        match answer(&request.method, request.params, self.0.clone()).await {
                            Ok(result) => Response::ok(request.id, result),
                            Err(error) => Response::failed(
                                Some(request.id),
                                bingo_jsonrpc::codec::RpcError {
                                    code: i64::from(i32::from(error.code)),
                                    message: error.message,
                                    data: error.data,
                                },
                            ),
                        };
                    let _ = sender.send(Message::Response(response));
                }
                Message::Response(_) => {}
            }
        }
        .boxed()
    }
}

/// A notification nothing can answer. An update whose shape this build does
/// not know — a variant a newer adapter ships ahead of the schema — is
/// dropped: a turn must not end because the agent said something extra.
async fn observe(method: &str, params: Value, client: &Arc<dyn Client>) {
    if let Ok(Incoming::Update(notification)) = method::incoming(method, params) {
        client.update(*notification).await;
    }
}

async fn answer(method: &str, params: Value, client: Arc<dyn Client>) -> Result<Value, RpcError> {
    match method::incoming(method, params) {
        Ok(Incoming::Permission(request)) => encoded(client.permission(*request).await),
        Ok(Incoming::Elicitation(request)) => encoded(client.elicitation(*request).await),
        // ADR-0035 §6: `fs/*` and `terminal/*` are declared unsupported, so
        // the agent is told rather than left waiting.
        Ok(_) => Err(RpcError::method_not_found()),
        Err(bad) => Err(RpcError::invalid_params().data(bad.to_string())),
    }
}

fn encoded<T: serde::Serialize>(answered: Result<T, RpcError>) -> Result<Value, RpcError> {
    answered.and_then(|answer| serde_json::to_value(answer).map_err(RpcError::into_internal_error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use agent_client_protocol_schema::v1::{
        InitializeRequest, RequestPermissionOutcome, SelectedPermissionOutcome, SessionUpdate,
    };
    use serde_json::json;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::io::{DuplexStream, Lines, ReadHalf, WriteHalf};
    use tokio::sync::Mutex;

    /// The far end of the duplex, standing in for a scripted agent.
    struct Agent {
        lines: Lines<BufReader<ReadHalf<DuplexStream>>>,
        writer: WriteHalf<DuplexStream>,
    }

    impl Agent {
        async fn read(&mut self) -> Value {
            let line = self
                .lines
                .next_line()
                .await
                .expect("the pipe is open")
                .expect("a line arrives");
            serde_json::from_str(&line).expect("the client writes JSON")
        }

        async fn write(&mut self, body: Value) {
            self.writer
                .write_all(format!("{body}\n").as_bytes())
                .await
                .expect("the pipe takes it");
        }

        fn hang_up(self) {
            drop(self);
        }
    }

    #[derive(Default)]
    struct Watcher {
        updates: Arc<Mutex<Vec<SessionNotification>>>,
        asked: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Client for Watcher {
        async fn update(&self, notification: SessionNotification) {
            self.updates.lock().await.push(notification);
        }

        async fn permission(
            &self,
            request: RequestPermissionRequest,
        ) -> Result<RequestPermissionResponse, RpcError> {
            self.asked.fetch_add(1, Ordering::Relaxed);
            let picked = request
                .options
                .first()
                .expect("the agent offers at least one option")
                .option_id
                .clone();
            Ok(RequestPermissionResponse::new(
                RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(picked)),
            ))
        }

        async fn elicitation(
            &self,
            _request: CreateElicitationRequest,
        ) -> Result<CreateElicitationResponse, RpcError> {
            self.asked.fetch_add(1, Ordering::Relaxed);
            Ok(crate::refusal::declined())
        }
    }

    fn pair() -> (Connection, Agent, Arc<Watcher>) {
        let (ours, theirs) = tokio::io::duplex(64 * 1024);
        let (our_read, our_write) = tokio::io::split(ours);
        let (their_read, their_write) = tokio::io::split(theirs);
        let watcher = Arc::new(Watcher::default());
        let connection = Connection::spawn(our_read, our_write, watcher.clone());
        let agent = Agent {
            lines: BufReader::new(their_read).lines(),
            writer: their_write,
        };
        (connection, agent, watcher)
    }

    #[tokio::test]
    async fn a_seventeen_mib_line_ends_the_connection_with_the_named_error() {
        let (connection, mut agent, _) = pair();
        let asking = tokio::spawn(async move {
            let request: InitializeRequest =
                serde_json::from_value(fixtures::initialize_request()).expect("request");
            connection.call(request).await
        });
        agent.read().await;
        let _ = agent.writer.write_all(&vec![b'x'; 17 * 1024 * 1024]).await;
        let error = tokio::time::timeout(std::time::Duration::from_secs(10), asking)
            .await
            .expect("bounded reader")
            .expect("call task")
            .expect_err("oversized line");
        assert!(
            matches!(error, AcpError::Transport(message) if message == "JSON-RPC line too long")
        );
    }

    #[tokio::test]
    async fn a_call_is_answered_by_its_id() {
        let (connection, mut agent, _) = pair();
        let asking = tokio::spawn(async move {
            let request: InitializeRequest =
                serde_json::from_value(fixtures::initialize_request()).expect("a request");
            connection.call(request).await
        });
        let sent = agent.read().await;
        assert_eq!(sent["method"], method::INITIALIZE);
        assert_eq!(sent["params"]["protocolVersion"], 1);
        agent
            .write(json!({
                "jsonrpc": "2.0",
                "id": sent["id"],
                "result": fixtures::initialize_response()
            }))
            .await;
        let answered = asking.await.expect("the task finishes").expect("an answer");
        assert!(answered.agent_capabilities.load_session);
    }

    /// Two questions in flight: the ids, not the order, decide which answer
    /// belongs to which.
    #[tokio::test]
    async fn answers_that_arrive_out_of_order_still_find_their_callers() {
        let (connection, mut agent, _) = pair();
        let connection = Arc::new(connection);
        let request = || -> InitializeRequest {
            serde_json::from_value(fixtures::initialize_request()).expect("a request")
        };
        let first = tokio::spawn({
            let connection = connection.clone();
            async move { connection.call(request()).await }
        });
        let one = agent.read().await;
        let second = tokio::spawn({
            let connection = connection.clone();
            async move { connection.call(request()).await }
        });
        let two = agent.read().await;
        assert_ne!(one["id"], two["id"], "each question gets its own id");
        agent
            .write(json!({ "jsonrpc": "2.0", "id": two["id"], "result": fixtures::initialize_response() }))
            .await;
        agent
            .write(json!({ "jsonrpc": "2.0", "id": one["id"], "error": { "code": -32000, "message": "no login" } }))
            .await;
        assert!(matches!(
            second.await.expect("the task finishes"),
            Ok(response) if response.agent_capabilities.load_session
        ));
        assert!(matches!(
            first.await.expect("the task finishes"),
            Err(AcpError::Refused(refusal)) if refusal.message == "no login"
        ));
    }

    #[tokio::test]
    async fn an_update_reaches_the_client_and_an_unknown_one_does_not_end_the_stream() {
        let (connection, mut agent, watcher) = pair();
        agent
            .write(json!({
                "jsonrpc": "2.0",
                "method": method::SESSION_UPDATE,
                "params": fixtures::update_agent_message_chunk()
            }))
            .await;
        // `subagent_spawned` is a draft RFD `codex-acp` ships ahead of the
        // schema. A turn must not end because the agent said something extra.
        agent
            .write(json!({
                "jsonrpc": "2.0",
                "method": method::SESSION_UPDATE,
                "params": fixtures::update_from_a_newer_adapter()
            }))
            .await;
        agent
            .write(json!({
                "jsonrpc": "2.0",
                "method": method::SESSION_UPDATE,
                "params": fixtures::update_agent_thought_chunk()
            }))
            .await;
        let seen = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if watcher.updates.lock().await.len() == 2 {
                    return;
                }
                tokio::task::yield_now().await;
            }
        })
        .await;
        assert!(
            seen.is_ok(),
            "both known updates arrive, the third is dropped"
        );
        let updates = watcher.updates.lock().await;
        assert!(matches!(
            updates[0].update,
            SessionUpdate::AgentMessageChunk(_)
        ));
        assert!(matches!(
            updates[1].update,
            SessionUpdate::AgentThoughtChunk(_)
        ));
        drop(connection);
    }

    /// Both doors an agent may knock on reach the client, and its answer goes
    /// back under the agent's own id. What the answer *is* is
    /// [`crate::refusal`]'s to decide; this is the routing.
    #[tokio::test]
    async fn a_question_the_agent_asks_reaches_the_client_and_is_answered_by_id() {
        let (connection, mut agent, watcher) = pair();
        agent
            .write(json!({
                "jsonrpc": "2.0",
                "id": "perm-1",
                "method": method::SESSION_REQUEST_PERMISSION,
                "params": fixtures::request_permission()
            }))
            .await;
        let reply = agent.read().await;
        assert_eq!(reply["id"], "perm-1", "the agent's own id comes back");
        assert_eq!(reply["result"], fixtures::request_permission_selected());

        agent
            .write(json!({
                "jsonrpc": "2.0",
                "id": "elicit-1",
                "method": method::ELICITATION_CREATE,
                "params": fixtures::elicitation_create()
            }))
            .await;
        let reply = agent.read().await;
        assert_eq!(reply["id"], "elicit-1");
        assert_eq!(reply["result"], fixtures::elicitation_declined());
        assert_eq!(watcher.asked.load(Ordering::Relaxed), 2);
        drop(connection);
    }

    /// ADR-0035 §6: this client has no filesystem and no terminal, and the
    /// agent is told so rather than left waiting.
    #[tokio::test]
    async fn a_method_this_client_does_not_have_is_refused_not_ignored() {
        let (connection, mut agent, _) = pair();
        agent
            .write(json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": method::FS_READ_TEXT_FILE,
                "params": { "sessionId": "s", "path": "/etc/passwd" }
            }))
            .await;
        let reply = agent.read().await;
        assert_eq!(reply["id"], 7);
        assert_eq!(reply["error"]["code"], -32601);
        drop(connection);
    }

    /// A dead adapter must fail the turn, not hold it open for ever.
    #[tokio::test]
    async fn a_call_whose_adapter_died_fails_instead_of_waiting() {
        let (connection, mut agent, _) = pair();
        let asking = tokio::spawn(async move {
            let request: InitializeRequest =
                serde_json::from_value(fixtures::initialize_request()).expect("a request");
            connection.call(request).await
        });
        agent.read().await;
        agent.hang_up();
        assert!(matches!(
            asking.await.expect("the task finishes"),
            Err(AcpError::Transport(_))
        ));
    }

    #[tokio::test]
    async fn a_notification_goes_out_with_no_id() {
        let (connection, mut agent, _) = pair();
        let cancel = serde_json::from_value(fixtures::cancel_notification()).expect("a cancel");
        connection
            .notify::<CancelNotification>(cancel)
            .expect("it goes");
        let sent = agent.read().await;
        assert_eq!(sent["method"], method::SESSION_CANCEL);
        assert!(sent.get("id").is_none(), "a notification has no id");
        drop(connection);
    }

    use agent_client_protocol_schema::v1::CancelNotification;
}
