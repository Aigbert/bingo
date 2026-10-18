#![allow(clippy::unwrap_used, clippy::expect_used)]
use bingo_sdk::{Event, Item, SessionState, TurnId, transcript::clock};
use jiff::Timestamp;
use serde_json::{Value, json};

fn at(ms: i64) -> Timestamp {
    Timestamp::from_millisecond(ms).unwrap()
}
fn state() -> SessionState {
    let Event::SessionUpdated { summary } =
        serde_json::from_str(include_str!("../fixtures/record/event/sessionUpdated.json")).unwrap()
    else {
        panic!("summary")
    };
    SessionState::new(summary)
}
fn item(state: &mut SessionState, id: &str, start: i64, end: i64, body: Value) {
    let item: Item = serde_json::from_value(json!({
        "id":id,"turn":"t","status":"completed","startedAt":at(start),
        "completedAt":at(end),"body":body
    }))
    .unwrap();
    state.transcript.apply(&Event::ItemCompleted { item });
}
fn live(state: &mut SessionState, turn: &str) {
    state.turn = Some(
        serde_json::from_value(json!({
            "id":turn,"startedAt":at(0),"origin":"submit"
        }))
        .unwrap(),
    );
}
fn read(state: &SessionState, now: i64) -> (i128, Option<Timestamp>) {
    let clock = clock(state, &TurnId::from_raw("t"), at(now)).unwrap();
    (clock.elapsed.as_millis(), clock.next_second)
}
fn prompt(state: &mut SessionState) {
    item(
        state,
        "prompt",
        100,
        100,
        json!({"kind":"assistant","text":"start"}),
    );
}
fn receipt(state: &mut SessionState, asked: Option<i64>) {
    let mut body =
        json!({"kind":"permissionReceipt","interaction":"ask","tool":"Edit","decision":"allow"});
    if let Some(asked) = asked {
        body["askedAt"] = json!(at(asked));
    }
    item(state, "receipt", 2350, 2350, body);
}
#[test]
fn no_asks_tick_on_elapsed_second_boundaries() {
    let mut s = state();
    prompt(&mut s);
    live(&mut s, "t");
    assert_eq!(read(&s, 1600), (1500, Some(at(2100))));
    assert_eq!(read(&s, 2100), (2000, Some(at(3100))));
}
#[test]
fn closed_asks_are_subtracted_and_shift_the_next_second() {
    let mut s = state();
    prompt(&mut s);
    receipt(&mut s, Some(600));
    live(&mut s, "t");
    assert_eq!(read(&s, 2600), (750, Some(at(2850))));
}
#[test]
fn an_open_ask_freezes_without_a_deadline() {
    let mut s = state();
    prompt(&mut s);
    live(&mut s, "t");
    s.interactions.push(serde_json::from_value(json!({
        "id":"ask","session":"s","turn":"t","openedAt":at(600),
        "kind":{"kind":"question","question":"Where?","options":[],"freeText":true,"multi":false},
        "answers":["text"]
    })).unwrap());
    assert_eq!(read(&s, 5000), (500, None));
}
#[test]
fn stopped_turns_end_at_the_last_completion() {
    let mut s = state();
    prompt(&mut s);
    receipt(&mut s, Some(600));
    item(
        &mut s,
        "stop",
        2600,
        2600,
        json!({"kind":"interruption","marker":"stopped"}),
    );
    assert_eq!(read(&s, 9000), (750, None));
}
#[test]
fn legacy_receipts_subtract_nothing() {
    let mut s = state();
    prompt(&mut s);
    receipt(&mut s, None);
    assert_eq!(read(&s, 9000), (2250, None));
}
#[test]
fn newest_and_older_finished_turns_read_alike() {
    let mut s = state();
    prompt(&mut s);
    receipt(&mut s, Some(600));
    let before = read(&s, 3000);
    live(&mut s, "next");
    assert_eq!(read(&s, 9000), before);
}
#[test]
fn a_missing_turn_has_no_clock() {
    assert!(clock(&state(), &TurnId::from_raw("missing"), at(0)).is_none());
}

#[test]
fn overlapping_question_waits_are_counted_once() {
    let mut s = state();
    prompt(&mut s);
    receipt(&mut s, Some(600));
    item(
        &mut s,
        "question",
        2500,
        2500,
        json!({
            "kind":"questionAnswer","interaction":"q","question":"Where?","answer":"Here","askedAt":at(1000)
        }),
    );
    live(&mut s, "t");
    assert_eq!(read(&s, 2600), (600, Some(at(3000))));
}

#[test]
fn malformed_waits_and_backwards_clocks_cannot_make_negative_elapsed() {
    let mut s = state();
    prompt(&mut s);
    receipt(&mut s, Some(9000));
    live(&mut s, "t");
    assert_eq!(read(&s, 50).0, 0);
    assert_eq!(read(&s, 2600).0, 2500);
}

#[test]
fn an_interruption_closes_the_open_wait_without_an_answer() {
    let mut s = state();
    prompt(&mut s);
    item(
        &mut s,
        "stop",
        2600,
        2600,
        json!({"kind":"interruption","marker":"stopped","askedAt":at(600)}),
    );
    assert_eq!(read(&s, 9000), (500, None));
}

#[test]
fn successive_answered_asks_subtract_both_waits() {
    let mut s = state();
    prompt(&mut s);
    receipt(&mut s, Some(600));
    item(
        &mut s,
        "question",
        4000,
        4000,
        json!({"kind":"questionAnswer","interaction":"q2","question":"Where?",
            "answer":"Here","askedAt":at(3000)}),
    );
    live(&mut s, "t");
    assert_eq!(read(&s, 4500), (1650, Some(at(4850))));
}

#[test]
fn indexed_and_session_clocks_agree_across_waits_and_finished_turns() {
    use bingo_sdk::transcript::{Ends, clock_of};
    let mut s = state();
    prompt(&mut s);
    receipt(&mut s, Some(600));
    let first = s.transcript.items().to_vec();
    for (turn, offset) in [("open", 3000), ("done", 6000)] {
        for original in &first {
            let mut item = original.as_ref().clone();
            item.id = bingo_sdk::ItemId::from_raw(format!("{turn}-{}", item.id));
            item.turn = Some(TurnId::from_raw(turn));
            item.started_at += jiff::SignedDuration::from_millis(offset);
            item.completed_at = Some(item.started_at);
            if let bingo_sdk::ItemBody::PermissionReceipt {
                asked_at: Some(asked),
                ..
            } = &mut item.body
            {
                *asked += jiff::SignedDuration::from_millis(offset);
            }
            s.transcript.apply(&Event::ItemCompleted { item });
        }
    }
    live(&mut s, "open");
    s.interactions.push(serde_json::from_value(json!({
        "id":"ask","session":"s","turn":"open","openedAt":at(5500),
        "kind":{"kind":"question","question":"Where?","options":[],"freeText":true,"multi":false},
        "answers":["text"]
    })).unwrap());
    for (turn, elapsed, next) in [
        ("t", 7150, Some(at(9850))),
        ("open", 650, None),
        ("done", 500, None),
    ] {
        if turn != "done" {
            live(&mut s, turn);
        }
        let id = TurnId::from_raw(turn);
        let items: Vec<_> = s
            .transcript
            .items()
            .iter()
            .filter(|item| item.turn.as_ref() == Some(&id))
            .map(AsRef::as_ref)
            .collect();
        for clock in [
            clock_of(&items, Ends::of(&s, &id), at(9000)),
            clock(&s, &id, at(9000)),
        ] {
            let clock = clock.unwrap();
            assert_eq!(
                (clock.elapsed.as_millis(), clock.next_second),
                (elapsed, next)
            );
        }
    }
}
