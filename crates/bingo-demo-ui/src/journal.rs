//! Where the board lives: the session's own journal, as the extension
//! `bingo.demo.ui`/`board` (ADR-0011 §2). The payload is the *view* a person
//! was shown, and the board reads straight back out of it — nothing is kept
//! between calls, and no shadow record has to be kept in step with the one
//! the journal carries.

use bingo_sdk::{
    ClientIdentity, ErrorCode, Event, Frame, Hook, HookContext, HookMatcher, HookPoint, HostHandle,
    ItemBody, ItemId, KernelError, Level, OpenOptions, SessionId, SessionSelector, SessionState,
    View,
};

use crate::board::Board;

/// The plugin this state belongs to, and the kinds within it.
pub const PLUGIN: &str = "bingo.demo.ui";
/// The journaled kind: back after `--continue`.
pub const BOARD: &str = "board";
/// The live kind: never journaled, gone on a resume (ADR-0013 §2).
pub const PROGRESS: &str = "progress";
pub const ANCHOR: &str = "DEMO_BOARD";

/// The board as the journal has it. A session that never published one, or
/// published something this crate cannot read, starts from a fresh board.
pub async fn read(host: &HostHandle, session: &SessionId) -> Result<Board, KernelError> {
    Ok(board_of(&snapshot(host, session).await?))
}

async fn snapshot(host: &HostHandle, session: &SessionId) -> Result<SessionState, KernelError> {
    let attachment = host
        .open(
            SessionSelector::ById {
                id: session.clone(),
            },
            ClientIdentity {
                name: "demo-ui".into(),
                surface: "demo-ui".into(),
            },
            OpenOptions::default(),
        )
        .await?;
    Ok(attachment.snapshot)
}

fn board_of(snapshot: &SessionState) -> Board {
    snapshot
        .extensions
        .get(PLUGIN)
        .and_then(|kinds| kinds.get(BOARD))
        .and_then(|payload| serde_json::from_value::<View>(payload.clone()).ok())
        .and_then(|view| Board::of_view(&view))
        .unwrap_or_default()
}

/// Publishes the whole board, which is what the kind means: the next snapshot
/// carries exactly this, and so does the next run that continues the session.
pub async fn write(
    host: &HostHandle,
    session: &SessionId,
    board: &Board,
) -> Result<Option<ItemId>, KernelError> {
    let state = snapshot(host, session).await?;
    let item = anchor(&state);
    let payload =
        bingo_sdk::slot::with(view(board)?, item.clone().map(|item| board.placement(item)));
    host.extend(session, PLUGIN, BOARD, payload).await?;
    Ok(item)
}

fn anchor(state: &SessionState) -> Option<ItemId> {
    state
        .transcript
        .items()
        .iter()
        .rev()
        .find(|item| is_anchor(&item.body))
        .map(|item| item.id.clone())
}

fn is_anchor(body: &ItemBody) -> bool {
    matches!(body, ItemBody::Notice { code, .. } if code == ANCHOR)
}

pub fn anchor_body() -> ItemBody {
    ItemBody::Notice {
        level: Level::Info,
        code: ANCHOR.into(),
        text: "Board".into(),
    }
}

/// A command's record id belongs to the kernel; place the row once it is published.
pub struct BoardRow;

#[async_trait::async_trait]
impl Hook for BoardRow {
    fn id(&self) -> &str {
        "demo-ui:board-row"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![HookPoint::Event],
            tool: None,
        }
    }
    async fn on_event(&self, frame: &Frame, cx: &HookContext) {
        if !matches!(&frame.event, Event::ItemCompleted { item } if is_anchor(&item.body)) {
            return;
        }
        if let Err(error) = place(&cx.host, &frame.session).await {
            let _ = cx
                .host
                .notice(Level::Warn, "BOARD_PLACEMENT_FAILED", &error.message)
                .await;
        }
    }
}

async fn place(host: &HostHandle, session: &SessionId) -> Result<(), KernelError> {
    let board = read(host, session).await?;
    write(host, session, &board).await?;
    Ok(())
}

fn view(board: &Board) -> Result<serde_json::Value, KernelError> {
    serde_json::to_value(board.view())
        .map_err(|e| KernelError::new(ErrorCode::Internal, format!("a view is json: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{board::State, command::TICK_COMMAND, tests::{Journals, command_context}};
    use bingo_sdk::Command;
    use std::sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}};

    #[tokio::test]
    async fn a_placement_cannot_overwrite_a_command_after_its_read() {
        let journals = Journals::new();
        let session = journals.session();
        write(&journals.handle(), &session, &Board::default()).await.unwrap();
        let placement_host = journals.scripted();
        let base = journals.handle();
        let (resume, paused) = tokio::sync::oneshot::channel();
        let paused = Arc::new(Mutex::new(Some(paused)));
        let reads = Arc::new(AtomicUsize::new(0));
        let observed = reads.clone();
        placement_host.on_open(move |selector, who, options| {
            let base = base.clone();
            let read = observed.fetch_add(1, Ordering::SeqCst);
            let pause = if read == 1 {paused.lock().unwrap().take()} else {None};
            async move {
                let attachment = base.open(selector, who, options).await?;
                if let Some(pause) = pause {pause.await.unwrap();}
                Ok(attachment)
            }
        });
        let handle = placement_host.handle();
        let placement = place(&handle, &session);
        tokio::pin!(placement);
        assert!(futures::poll!(&mut placement).is_pending());
        assert_eq!(reads.load(Ordering::SeqCst), 2, "placement read the old board before pausing at its write");
        let cx = command_context(&session, &journals);
        let command = TICK_COMMAND.run("", &cx);
        tokio::pin!(command);
        // Run the newer command while placement still holds its old snapshot.
        // A serialized owner may hold its write until placement is released.
        let completed = match futures::poll!(&mut command) {
            std::task::Poll::Ready(result) => {result.unwrap(); true}
            std::task::Poll::Pending => false,
        };
        resume.send(()).unwrap();
        placement.await.unwrap();
        if !completed {command.await.unwrap();}
        assert_eq!(read(&journals.handle(), &session).await.unwrap().rows[0].state, State::Running);
    }
    #[tokio::test]
    async fn a_queued_write_keeps_the_owner_until_its_frame_is_published() {
        let journals = Journals::new();
        let session = journals.session();
        let host = journals.scripted();
        let (sent, mut received) = tokio::sync::mpsc::unbounded_channel();
        host.on_extend(move |session, plugin, kind, value| {
            sent.send((session, plugin, kind, value)).unwrap();
            async {Ok(())}
        });
        let mut board = Board::default();
        board.tick();
        let handle = host.handle();
        let writing = write(&handle, &session, &board);
        tokio::pin!(writing);
        assert!(futures::poll!(&mut writing).is_pending(), "enqueueing is not publication");
        let base = journals.handle();
        let placement = place(&base, &session);
        tokio::pin!(placement);
        assert!(futures::poll!(&mut placement).is_pending(), "placement must wait for the preceding board write");
        let (id, plugin, kind, value) = received.try_recv().unwrap();
        base.extend(&id, &plugin, &kind, value).await.unwrap();
        writing.await.unwrap();
        placement.await.unwrap();
        assert_eq!(read(&base, &session).await.unwrap(), board);
    }

}
