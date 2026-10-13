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
    for invalid in ["Parent", "PARENT", "parent", "#room", "", "a/b", "two words"] {
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
