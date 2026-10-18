//! Screen stories fold real turn boundaries; unit fixtures may still name a turn explicitly.
use super::*;

pub fn session(frames: Vec<Frame>) -> SessionState {
    if frames
        .iter()
        .any(|f| matches!(f.event, Event::TurnStarted { .. }))
    {
        return folded(frames);
    }
    let mut state = state();
    let mut active: Option<TurnId> = None;
    let mut seconds = 0;
    let mut began = 0;
    for mut next in frames {
        let prompt = matches!(&next.event, Event::ItemCompleted { item } | Event::ItemStarted { item }
            if matches!(&item.body, ItemBody::User { origin, .. }
                if origin.principal.is_none() && bingo_sdk::effective_voice(origin) == bingo_sdk::Voice::Said));
        if prompt {
            finish(&mut state, active.take(), began + 8);
            if !state.transcript.items().is_empty() {
                seconds = seconds.max(began + 10);
            }
            began = seconds;
            let turn = TurnId::from_raw(format!("scene_{seconds}"));
            apply(&mut state, started(turn.as_str()), seconds);
            active = Some(turn);
        }
        if let Event::ItemCompleted { item } | Event::ItemStarted { item } = &mut next.event {
            item.turn = active.clone();
            item.started_at = ts() + jiff::SignedDuration::from_secs(seconds);
            item.completed_at = item.status.is_terminal().then_some(item.started_at);
            seconds += 1;
        }
        if let Event::TurnCompleted { turn, status, .. } = &mut next.event {
            if let Some(current) = active.take() {
                *turn = current;
            }
            seconds = began
                + if matches!(status, bingo_sdk::TurnStatus::Interrupted { .. }) {
                    7
                } else {
                    8
                };
        }
        apply(&mut state, next.event, seconds);
    }
    if state
        .transcript
        .items()
        .iter()
        .all(|i| i.status.is_terminal())
    {
        finish(&mut state, active, began + 8);
    }
    state
}

fn apply(state: &mut SessionState, event: Event, seconds: i64) {
    let mut next = frame(state.seq.0 + 1, event);
    next.ts = ts() + jiff::SignedDuration::from_secs(seconds);
    state.apply(&next);
}

fn finish(state: &mut SessionState, turn: Option<TurnId>, seconds: i64) {
    if let Some(turn) = turn {
        if let Some(item) = state
            .transcript
            .items()
            .iter()
            .rev()
            .find(|item| item.turn.as_ref() == Some(&turn))
        {
            let mut item = item.as_ref().clone();
            item.completed_at = Some(ts() + jiff::SignedDuration::from_secs(seconds));
            apply(state, Event::ItemCompleted { item }, seconds);
        }
        apply(
            state,
            completed(turn.as_str(), bingo_sdk::TurnStatus::Completed),
            seconds,
        );
    }
}
