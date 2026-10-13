#![allow(clippy::unwrap_used, clippy::expect_used)]
use bingo_collab::{Name, room};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

#[test]
fn names_keep_their_spelling_and_share_one_rule() {
    for (a, b, equal) in [("Scout", "scout", true), ("scout", "scouts", false)] {
        let left = Name::parse(a).unwrap();
        assert_eq!(left.to_string(), a);
        assert_eq!(left == Name::parse(b).unwrap(), equal);
    }
    for invalid in [
        "Parent",
        "PARENT",
        "parent",
        "#room",
        "",
        "a/b",
        "two words",
    ] {
        assert!(Name::parse(invalid).is_err(), "{invalid}");
    }
    assert_eq!(Name::parse(" Scout ").unwrap().to_string(), "Scout");
}

fn round_trip<T: Serialize + DeserializeOwned>(source: &str) {
    let expected: Value = serde_json::from_str(source).unwrap();
    let record: T = serde_json::from_value(expected.clone()).unwrap();
    assert_eq!(serde_json::to_value(record).unwrap(), expected);
}

#[test]
fn journal_record_shapes_stay_compatible() {
    round_trip::<room::Opened>(include_str!("../fixtures/opened.json"));
    round_trip::<room::Closed>(include_str!("../fixtures/closed.json"));
    round_trip::<room::Members>(include_str!("../fixtures/members.json"));
    round_trip::<room::Retuned>(include_str!("../fixtures/ear.json"));
    round_trip::<room::Cursor>(include_str!("../fixtures/cursor.json"));
    round_trip::<room::Listener>(include_str!("../fixtures/listener.json"));
}

fn state() -> bingo_sdk::SessionState {
    let event: bingo_sdk::Event = serde_json::from_str(include_str!("../../bingo-sdk/fixtures/record/event/sessionUpdated.json")).unwrap();
    let bingo_sdk::Event::SessionUpdated { summary } = event else { panic!("summary") };
    bingo_sdk::SessionState::new(summary)
}

#[test]
fn room_readers_preserve_the_columns_and_targets_from_fixtures() {
    let mut state = state();
    let records = state.extensions.entry(room::PLUGIN.into()).or_default();
    records.insert(room::MEMBERS.into(), serde_json::from_str(include_str!("../fixtures/members.json")).unwrap());
    records.insert("ear:scout".into(), serde_json::from_str(include_str!("../fixtures/ear.json")).unwrap());
    records.insert("cursor:scout".into(), serde_json::from_str(include_str!("../fixtures/cursor.json")).unwrap());
    assert_eq!(room::members_of(&state), ["scout"]);
    assert_eq!(room::HOLDER, "parent");
    assert_eq!(room::hearing(&state, "Scout"), room::Hearing::Listening { patience_s: Some(120) });
    assert_eq!(room::unread_count(&state, "scout"), None);
    let event: bingo_sdk::Event = serde_json::from_str(include_str!("../../bingo-sdk/fixtures/record/event/itemCompleted.json")).unwrap();
    let bingo_sdk::Event::ItemCompleted { item } = event else { panic!("item") };
    for (seq, id) in [(1, "itm_7"), (2, "itm_8")] {
        let mut item = item.clone();
        item.id = bingo_sdk::ItemId::from_raw(id);
        state.apply(&bingo_sdk::Frame { seq: bingo_sdk::Seq(seq), ts: item.started_at, session: state.summary.id.clone(), cause: None, event: bingo_sdk::Event::ItemCompleted { item } });
    }
    assert_eq!(room::unread_count(&state, "SCOUT"), Some(1));
    assert_eq!(room::unread_count(&state, "missing"), None);
}
