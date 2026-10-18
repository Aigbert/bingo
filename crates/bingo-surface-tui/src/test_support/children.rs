//! Child stories use their own turn and clock on the shared event stream.
use super::*;

pub fn child_frame(seq: u64, mut event: Event) -> Frame {
    let time = ts() + jiff::SignedDuration::from_secs(seq.saturating_sub(2) as i64);
    if let Event::ItemStarted { item } | Event::ItemCompleted { item } = &mut event {
        item.turn = Some(TurnId::from_raw("trn_9"));
        item.started_at = time;
        item.completed_at = item.status.is_terminal().then_some(time);
    }
    Frame {
        session: child_id(),
        ts: time,
        ..frame(seq, event)
    }
}

/// A sub-session that has run a while: three tool calls and some tokens
/// spent, which is what its row in the parent's transcript reports.
pub fn busy_child(title: &str) -> Vec<Frame> {
    let mut summary = child_summary(title);
    summary.usage = Usage {
        input_tokens: 1_100,
        output_tokens: 140,
        ..Usage::default()
    };
    let mut frames = vec![
        child_frame(1, Event::SessionUpdated { summary }),
        child_frame(2, started("trn_9")),
        child_frame(
            3,
            Event::ItemCompleted {
                item: user("itm_prompt", "review the diff"),
            },
        ),
    ];
    for i in 1..=3 {
        frames.push(child_frame(
            3 + i,
            Event::ItemCompleted {
                item: tool(
                    &format!("itm_{i}"),
                    "Read",
                    json!({ "file_path": "src/lib.rs" }),
                    Some(ToolOutput::text("Read 3 lines")),
                    ItemStatus::Completed,
                ),
            },
        ));
    }
    frames
}
