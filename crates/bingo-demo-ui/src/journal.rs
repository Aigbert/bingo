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
