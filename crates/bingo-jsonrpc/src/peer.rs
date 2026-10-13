use crate::codec::{CodecError, Id, MAX_LINE, Message, Notification, Request, RpcError, lines};
use crate::pending::Pending;
use futures::{SinkExt, StreamExt, future::BoxFuture};
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::codec::{FramedRead, FramedWrite};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum PeerError {
    #[error("the connection closed")]
    Closed,
    #[error("JSON-RPC line too long")]
    LineTooLong,
    #[error("{0}")]
    Io(String),
    #[error("{}", .0.message)]
    Remote(RpcError),
}

impl From<CodecError> for PeerError {
    fn from(error: CodecError) -> Self {
        match error {
            CodecError::LineTooLong => Self::LineTooLong,
            CodecError::Io(error) => Self::Io(error.to_string()),
        }
    }
}

/// Protocol handlers own notifications, inbound requests and unmatched replies.
pub trait Handler: Send + Sync + 'static {
    fn handle(&self, message: Message, sender: Sender) -> BoxFuture<'_, ()>;
    fn closed(&self, _error: &PeerError) {}
}

#[derive(Clone)]
pub struct Sender {
    queue: mpsc::UnboundedSender<Message>,
    pending: Pending,
}

impl Sender {
    pub fn send(&self, message: Message) -> Result<(), PeerError> {
        let state = self.pending.lock();
        if let Some(error) = &state.error {
            return Err(error.clone());
        }
        self.queue.send(message).map_err(|_| PeerError::Closed)
    }
}

struct Shared {
    pending: Pending,
    handler: Arc<dyn Handler>,
    stop: CancellationToken,
}

impl Shared {
    fn close(&self, error: PeerError) {
        if self.pending.close(error.clone()) {
            self.handler.closed(&error);
        }
        self.stop.cancel();
    }
}

pub struct Peer {
    shared: Arc<Shared>,
    sender: Sender,
    next: AtomicI64,
    tasks: Vec<JoinHandle<()>>,
}

impl Peer {
    pub fn spawn<R, W>(reader: R, writer: W, handler: Arc<dyn Handler>) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (queue, rx) = mpsc::unbounded_channel();
        let shared = Arc::new(Shared {
            pending: Pending::default(),
            handler,
            stop: CancellationToken::new(),
        });
        let sender = Sender {
            queue,
            pending: shared.pending.clone(),
        };
        let tasks = vec![
            tokio::spawn(read(reader, shared.clone(), sender.clone())),
            tokio::spawn(write(writer, rx, shared.clone())),
        ];
        Self {
            shared,
            sender,
            next: AtomicI64::new(1),
            tasks,
        }
    }

    pub fn next_id(&self) -> Id {
        Id::Number(self.next.fetch_add(1, Ordering::Relaxed))
    }

    pub fn is_alive(&self) -> bool {
        self.shared.pending.lock().error.is_none()
    }

    pub fn close(&self) {
        self.shared.close(PeerError::Closed);
    }

    pub async fn request(&self, method: &str, params: Value) -> Result<Value, PeerError> {
        let id = self.next_id();
        let (_guard, rx) = self.shared.pending.register(id.clone())?;
        self.sender
            .send(Message::Request(Request::new(id, method, params)))?;
        rx.await.map_err(|_| PeerError::Closed)?
    }

    pub fn notify(&self, method: &str, params: Value) -> Result<(), PeerError> {
        self.sender
            .send(Message::Notification(Notification::new(method, params)))
    }

    pub fn fire(&self, method: &str, params: Value) -> Result<(), PeerError> {
        self.sender.send(Message::Request(Request::new(
            self.next_id(),
            method,
            params,
        )))
    }

    #[cfg(test)]
    pub(crate) fn pending_count(&self) -> usize {
        self.shared.pending.lock().waiting.len()
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        self.close();
        for task in &self.tasks {
            task.abort();
        }
    }
}

async fn read(reader: impl AsyncRead + Unpin, shared: Arc<Shared>, sender: Sender) {
    let mut input = FramedRead::new(reader, lines(MAX_LINE));
    let error = loop {
        let line = tokio::select! {
            () = shared.stop.cancelled() => return,
            line = input.next() => line,
        };
        let line = match line {
            Some(Ok(line)) => line,
            Some(Err(error)) => break error.into(),
            None => break PeerError::Closed,
        };
        let Ok(message) = serde_json::from_str::<Message>(&line) else {
            continue;
        };
        let message = match message {
            Message::Response(response) => match shared.pending.settle(response) {
                Some(response) => Message::Response(response),
                None => continue,
            },
            message => message,
        };
        tokio::select! {
            () = shared.stop.cancelled() => return,
            () = shared.handler.handle(message, sender.clone()) => {},
        }
    };
    shared.close(error);
}

async fn write(
    writer: impl AsyncWrite + Unpin,
    mut rx: mpsc::UnboundedReceiver<Message>,
    shared: Arc<Shared>,
) {
    let mut output = FramedWrite::new(writer, lines(MAX_LINE));
    let sending = async {
        while let Some(message) = rx.recv().await {
            let line = message
                .line()
                .map_err(|error| PeerError::Io(error.to_string()))?;
            output.send(line).await.map_err(PeerError::from)?;
        }
        Err::<(), _>(PeerError::Closed)
    };
    tokio::select! {
        () = shared.stop.cancelled() => {},
        Err(error) = sending => shared.close(error),
    }
}
