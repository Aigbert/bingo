//! The client half of one conversation: where a turn's stream goes, and what
//! the agent is told when it asks this client something.
//!
//! The adapter's row speaks first. Permission requests that reach bingo pass
//! the running turn's judge; only its verdict selects an ACP option. Outside
//! a turn they fail closed. Free-form elicitation remains unsupported.

use std::sync::atomic::{AtomicBool, Ordering};

use agent_client_protocol_schema::v1::{
    CreateElicitationRequest, CreateElicitationResponse, Error as RpcError,
    RequestPermissionRequest, RequestPermissionResponse, SessionNotification,
};
use async_trait::async_trait;
use bingo_sdk::{HostHandle, SessionId};
use tokio::sync::{Mutex, mpsc};

use crate::connection::Client;
use crate::{permission, refusal};
use bingo_sdk::{AgentEvent, CancellationToken, Judge, ProviderError, Verdict};
use std::sync::Arc;

/// Where a running turn's stream goes.
type Sink = mpsc::UnboundedSender<SessionNotification>;

struct Judging {
    judge: Arc<dyn Judge>,
    errors: mpsc::UnboundedSender<Result<AgentEvent, ProviderError>>,
    cancel: CancellationToken,
}

pub struct Inbox {
    judging: Mutex<Option<Judging>>,
    adapter: String,
    sink: Mutex<Option<Sink>>,
    /// True while `session/load` replays. The journal already holds those
    /// turns; writing them again would be the conversation twice.
    loading: AtomicBool,
    /// Whether this adapter has already been told, once, that it asked where
    /// nobody could answer.
    told: AtomicBool,
    host: Option<HostHandle>,
}

impl Inbox {
    pub fn new(adapter: &str, host: Option<HostHandle>, _session: Option<&SessionId>) -> Self {
        Inbox {
            judging: Mutex::new(None),
            adapter: adapter.to_string(),
            sink: Mutex::new(None),
            loading: AtomicBool::new(false),
            told: AtomicBool::new(false),
            host,
        }
    }

    pub fn adapter(&self) -> &str {
        &self.adapter
    }

    /// Take the stream of the turn that is starting. The previous turn's sink,
    /// if any, is dropped with it.
    pub async fn listen(&self) -> mpsc::UnboundedReceiver<SessionNotification> {
        let (sink, updates) = mpsc::unbounded_channel();
        *self.sink.lock().await = Some(sink);
        updates
    }

    /// Whether what arrives now is a replay of turns the journal already holds.
    pub fn replaying(&self, loading: bool) {
        self.loading.store(loading, Ordering::Release);
    }

    /// The question, put to whoever is at this session. `None` when nothing
    /// chose an option: no session behind this conversation, a door that
    /// refused it, or a surface that declined what it was handed.
    pub async fn judge(
        &self,
        judge: Arc<dyn Judge>,
        errors: mpsc::UnboundedSender<Result<AgentEvent, ProviderError>>,
        cancel: CancellationToken,
    ) {
        *self.judging.lock().await = Some(Judging {
            judge,
            errors,
            cancel,
        });
    }
    pub async fn clear_judge(&self) {
        self.judging.lock().await.take();
    }

    async fn put(&self, request: &RequestPermissionRequest) -> Option<RequestPermissionResponse> {
        let judging = self.judging.lock().await;
        let judging = judging.as_ref()?;
        let verdict = tokio::select! {
            biased;
            _ = judging.cancel.cancelled() => Verdict::Deny { feedback: None },
            verdict = judging.judge.judge(permission::call(request)) => verdict,
        };
        let option = permission::narrowest(request, &verdict);
        if option.is_none() {
            let _ = judging.errors.send(Err(ProviderError::Request {
                message: "agent permission request offered no reject option".into(),
            }));
        }
        option.map(permission::selected)
    }

    /// The one thing this plugin must not do silently: an agent asked, and the
    /// answer was made by a rule the person never sees unless it is said. Said
    /// once — an agent may ask on every call it makes.
    async fn say_where_the_answer_lives(&self) {
        if self.told.swap(true, Ordering::AcqRel) {
            return;
        }
        let Some(host) = self.host.as_ref() else {
            return;
        };
        let (level, code, text) = refusal::told(&self.adapter);
        let _ = host.notice(level, &code, &text).await;
    }
}

#[async_trait]
impl Client for Inbox {
    async fn update(&self, notification: SessionNotification) {
        if self.loading.load(Ordering::Acquire) {
            return;
        }
        if let Some(sink) = self.sink.lock().await.as_ref() {
            let _ = sink.send(notification);
        }
    }

    /// One `session/request_permission` is one question (ADR-0039 §3). The
    /// agent always gets an answer: the option that was chosen, or — when none
    /// was — its own refusal, and one line to the person saying so.
    async fn permission(
        &self,
        request: RequestPermissionRequest,
    ) -> Result<RequestPermissionResponse, RpcError> {
        if let Some(answered) = self.put(&request).await {
            return Ok(answered);
        }
        self.say_where_the_answer_lives().await;
        Ok(refusal::refused(&request))
    }

    async fn elicitation(
        &self,
        _request: CreateElicitationRequest,
    ) -> Result<CreateElicitationResponse, RpcError> {
        self.say_where_the_answer_lives().await;
        Ok(refusal::declined())
    }
}
