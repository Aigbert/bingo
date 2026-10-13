//! The actor fixes a cut; storage and folding happen on owned work off it.

use super::{Actor, Msg, history::History};
use bingo_sdk::*;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::oneshot;

type Attached = (SessionState, Option<OpenHistory>, FrameStream);

pub(crate) enum Request {
    Page(
        HistoryPage,
        oneshot::Sender<Result<HistoryChunk, KernelError>>,
    ),
    Pin(ItemId, u64, oneshot::Sender<Result<Item, KernelError>>),
    Attach(Box<SessionState>, FrameStream, oneshot::Sender<Attached>),
    Rewind(TurnId, oneshot::Sender<Result<u32, KernelError>>),
}

pub(super) fn attach_read(
    actor: &mut Actor,
    budget: Option<usize>,
    reply: oneshot::Sender<Attached>,
) {
    let (snapshot, history, stream) = actor.attached(budget);
    if budget.is_none() && actor.state.transcript.history().has_more {
        read_past(actor, Request::Attach(Box::new(snapshot), stream, reply));
    } else {
        let _ = reply.send((snapshot, history, stream));
    }
}

pub(super) fn history_read(
    actor: &Actor,
    page: HistoryPage,
    reply: oneshot::Sender<Result<HistoryChunk, KernelError>>,
) {
    if actor.state.transcript.history().has_more {
        read_past(actor, Request::Page(page, reply));
    } else {
        let _ = reply.send(History(&actor.state.transcript).history(&page));
    }
}

pub(super) fn pin_read(
    actor: &Actor,
    id: ItemId,
    generation: u64,
    reply: oneshot::Sender<Result<Item, KernelError>>,
) {
    if actor.state.transcript.history().has_more {
        read_past(actor, Request::Pin(id, generation, reply));
    } else {
        let _ = reply.send(History(&actor.state.transcript).item_for_pin(&id, generation));
    }
}

pub(super) async fn rewind_read(
    actor: &mut Actor,
    turn: TurnId,
    reply: oneshot::Sender<Result<u32, KernelError>>,
) {
    if actor.state.transcript.history().has_more {
        read_past(actor, Request::Rewind(turn, reply));
    } else {
        let transcript = actor.state.transcript.clone();
        let _ = reply.send(actor.rewind(&transcript, &turn).await);
    }
}

fn read_past(actor: &Actor, request: Request) {
    let Some(store) = actor.store.clone() else {
        return;
    };
    let session = actor.id.clone();
    let cut = actor.state.seq;
    let window = actor.state.transcript.clone();
    let generation = window.generation();
    let mailbox = actor.mailbox.clone();
    actor.tracker.spawn(async move {
        let transcript = read(store, session, cut, window).await;
        mailbox.send(Msg::PastLoaded {
            cut,
            generation,
            transcript,
            request: Box::new(request),
        });
    });
}

pub(super) async fn past_loaded(
    actor: &mut Actor,
    cut: Seq,
    generation: u64,
    transcript: Result<Transcript, KernelError>,
    request: Request,
) {
    if let Request::Attach(mut snapshot, stream, reply) = request {
        match transcript {
            Ok(transcript) => {
                snapshot.transcript = transcript;
                let _ = reply.send((*snapshot, None, stream));
            }
            Err(error) => {
                tracing::warn!(session = %actor.id, %error, "unbounded attachment history failed")
            }
        }
        return;
    }
    let transcript = History(&actor.state.transcript)
        .check_generation(Some(generation))
        .and(transcript);
    match request {
        Request::Page(page, reply) => {
            let _ = reply.send(transcript.and_then(|t| History(&t).history(&page)));
        }
        Request::Pin(id, generation, reply) => {
            let _ = reply.send(transcript.and_then(|t| History(&t).item_for_pin(&id, generation)));
        }
        Request::Rewind(turn, reply) => {
            let result = match transcript {
                Ok(transcript) if cut == actor.state.seq => actor.rewind(&transcript, &turn).await,
                Ok(_) => Err(KernelError::new(
                    ErrorCode::StaleGeneration,
                    "history changed during rewind",
                )),
                Err(error) => Err(error),
            };
            let _ = reply.send(result);
        }
        Request::Attach(..) => {}
    }
}

async fn read(
    store: Arc<dyn SessionStore>,
    session: SessionId,
    cut: Seq,
    window: Transcript,
) -> Result<Transcript, KernelError> {
    let frames = store.replay(&session, Seq::ZERO).await?;
    tokio::task::spawn_blocking(move || {
        let mut transcript = Transcript::new();
        for frame in frames.iter().filter(|frame| frame.seq <= cut) {
            transcript.apply(&frame.event);
        }
        window.with_history(transcript)
    })
    .await
    .map_err(|error| {
        KernelError::new(
            ErrorCode::Storage,
            format!("history worker failed: {error}"),
        )
    })
}

pub(super) fn catch_up(
    store: Arc<dyn SessionStore>,
    session: SessionId,
    since: Seq,
    cut: Seq,
    live: FrameStream,
) -> FrameStream {
    Box::pin(
        futures::stream::once(async move {
            match store.replay(&session, since).await {
                Ok(frames) => Box::pin(
                    futures::stream::iter(frames.into_iter().filter(move |f| f.seq < cut))
                        .chain(live),
                ) as FrameStream,
                Err(error) => {
                    tracing::warn!(%session, %error, "stored catch-up failed");
                    Box::pin(futures::stream::iter([Frame {
                        seq: cut,
                        ts: jiff::Timestamp::now(),
                        session,
                        cause: None,
                        event: Event::Lagged {
                            from: since.next(),
                            to: cut,
                        },
                    }])) as FrameStream
                }
            }
        })
        .flatten(),
    )
}
