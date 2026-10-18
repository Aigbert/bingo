#![allow(clippy::unwrap_used, clippy::expect_used)]
use bingo_sdk::{
    Event, Item, ItemBody, ItemStatus, Transcript,
    transcript::{Ends, call_wait, clock_of},
};
use jiff::Timestamp;
use serde_json::json;

fn at(ms: i64) -> Timestamp {
    Timestamp::from_millisecond(ms).unwrap()
}
fn call() -> Item {
    serde_json::from_value(json!({"id":"call","turn":"t","status":"pending",
        "startedAt":at(1000),"body":{"kind":"toolCall","callId":"c","name":"Edit","input":{}}}))
    .unwrap()
}
fn prompt() -> Item {
    serde_json::from_value(json!({"id":"prompt","turn":"t","status":"completed",
        "startedAt":at(0),"completedAt":at(0),"body":{"kind":"assistant","text":"go"}}))
    .unwrap()
}
fn stamp(item: &mut Item, asked: i64, began: Option<i64>) {
    let ItemBody::ToolCall {
        asked_at, began_at, ..
    } = &mut item.body
    else {
        panic!("call")
    };
    *asked_at = Some(at(asked));
    *began_at = began.map(at);
}
fn measured(transcript: &Transcript, ends: Ends, now: i64) -> (i128, Option<Timestamp>) {
    let items: Vec<_> = transcript.items().iter().map(AsRef::as_ref).collect();
    let clock = clock_of(&items, ends, at(now)).unwrap();
    (clock.elapsed.as_millis(), clock.next_second)
}
#[test]
fn a_calls_wait_spans_the_answer_until_running_frame_by_frame() {
    let mut transcript = Transcript::new();
    transcript.apply(&Event::ItemCompleted { item: prompt() });
    let mut item = call();
    transcript.apply(&Event::ItemStarted { item: item.clone() });
    assert_eq!(
        measured(&transcript, Ends::Live { asked: None }, 1000),
        (1000, Some(at(2000)))
    );
    stamp(&mut item, 2000, None);
    transcript.apply(&Event::ItemUpdated { item: item.clone() });
    assert_eq!(
        measured(&transcript, Ends::Live { asked: None }, 2000),
        (2000, None)
    );
    assert_eq!(
        measured(
            &transcript,
            Ends::Live {
                asked: Some(at(2000))
            },
            5000
        ),
        (2000, None)
    );
    assert_eq!(
        measured(&transcript, Ends::Live { asked: None }, 5500),
        (2000, None)
    );
    stamp(&mut item, 2000, Some(6000));
    item.status = ItemStatus::Running;
    transcript.apply(&Event::ItemUpdated { item: item.clone() });
    assert_eq!(
        measured(&transcript, Ends::Live { asked: None }, 6000),
        (2000, Some(at(7000)))
    );
    assert_eq!(
        measured(&transcript, Ends::Live { asked: None }, 7500),
        (3500, Some(at(8000)))
    );
    item.status = ItemStatus::Completed;
    item.completed_at = Some(at(8000));
    transcript.apply(&Event::ItemCompleted { item });
    assert_eq!(measured(&transcript, Ends::Over, 9000), (4000, None));
}
#[test]
fn completed_and_interrupted_calls_close_their_own_waits() {
    for (began, completed, elapsed) in [(Some(6000), 7000, 4000), (None, 9000, 2000)] {
        let mut transcript = Transcript::new();
        transcript.apply(&Event::ItemCompleted { item: prompt() });
        let mut item = call();
        stamp(&mut item, 2000, began);
        item.completed_at = Some(at(completed));
        item.status = if began.is_some() {
            ItemStatus::Completed
        } else {
            ItemStatus::Interrupted
        };
        assert_eq!(
            call_wait(&item),
            Some((at(2000), Some(at(began.unwrap_or(completed)))))
        );
        transcript.apply(&Event::ItemCompleted { item });
        let mut last = prompt();
        last.id = bingo_sdk::ItemId::from_raw("last");
        last.started_at = at(completed.max(8000));
        last.completed_at = Some(last.started_at);
        last.body = ItemBody::Interruption {
            marker: "stopped".into(),
            asked_at: None,
        };
        transcript.apply(&Event::ItemCompleted { item: last });
        assert_eq!(measured(&transcript, Ends::Over, 10000), (elapsed, None));
    }
}

#[test]
fn real_frames_never_restart_the_clock_between_answer_and_start() {
    let Event::SessionUpdated { summary } =
        serde_json::from_str(include_str!("../fixtures/record/event/sessionUpdated.json")).unwrap()
    else {
        panic!("summary")
    };
    let mut state = bingo_sdk::SessionState::new(summary);
    let mut call = call();
    let streamed = Event::ItemStarted { item: call.clone() };
    stamp(&mut call, 2000, None);
    let stamped = Event::ItemUpdated { item: call.clone() };
    stamp(&mut call, 2000, Some(6000));
    call.status = ItemStatus::Running;
    let began = Event::ItemUpdated { item: call.clone() };
    call.completed_at = Some(at(7000));
    call.status = ItemStatus::Completed;
    let opened = serde_json::from_value(json!({"type":"interactionOpened","interaction":{
        "id":"ask","session":"s","turn":"t","item":"call","openedAt":at(2000),
        "kind":{"kind":"permission","tool":"Edit","input":{},"summary":"Allow?"},"answers":["allowOnce"]}})).unwrap();
    let resolved = serde_json::from_value(json!({"type":"interactionResolved","id":"ask",
        "answer":{"kind":"allowOnce"},"by":{"kind":"client","name":"test","surface":"test"}}))
    .unwrap();
    let turn = bingo_sdk::TurnId::from_raw("t");
    let events = [
        (
            Event::TurnStarted {
                turn: turn.clone(),
                inputs: vec![],
                origin: bingo_sdk::TurnOrigin::Submit,
            },
            0,
            0,
        ),
        (Event::ItemCompleted { item: prompt() }, 0, 0),
        (streamed, 1000, 1000),
        (stamped, 2000, 2000),
        (opened, 5000, 2000),
        (resolved, 5500, 2000),
        (began, 6000, 2000),
        (Event::ItemCompleted { item: call }, 7000, 3000),
    ];
    for (index, (event, now, elapsed)) in events.into_iter().enumerate() {
        state.apply(&bingo_sdk::Frame {
            seq: bingo_sdk::Seq(index as u64 + 1),
            ts: at(now),
            session: state.summary.id.clone(),
            cause: None,
            event,
        });
        if index > 0 {
            let measured = bingo_sdk::transcript::clock(&state, &turn, at(now)).unwrap();
            assert_eq!(measured.elapsed.as_millis(), elapsed, "frame {index}");
        }
    }
}
