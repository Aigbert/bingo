//! Publish the shared owed view.
pub use bingo_collab::owed::*;
use bingo_sdk::{HostHandle, SessionId};
use serde_json::Value;
use crate::PLUGIN;

/// Put it where a person looks: on the session the room hangs under, which is
/// the whole of who its members are.
pub async fn publish(host: &HostHandle, parent: &SessionId, payload: Value) {
    if let Err(error) = host.signal(parent, PLUGIN, KIND, payload).await {
        tracing::debug!(%error, "what the rooms owe was not published");
    }
}

#[cfg(test)]
mod tests {
use crate::mentions::Mention;
use bingo_sdk::View;
use jiff::Timestamp;
    use super::*;
    use crate::mentions::Owed;
    use bingo_sdk::ItemId;

    fn at(second: i64) -> Timestamp {
        Timestamp::from_second(second).expect("a timestamp")
    }

    fn mention(owed_by: Owed, second: i64) -> Mention {
        Mention {
            owed_by,
            asker: "parent".into(),
            post: ItemId::from_raw(format!("itm_{second}")),
            at: at(second),
            head: "look again".into(),
        }
    }

    fn member(name: &str, second: i64) -> Mention {
        mention(Owed::Member(name.into()), second)
    }

    #[test]
    fn the_column_names_who_owes_oldest_first_with_how_long() {
        let open = [member("scout", 60), member("reviewer", 0)];
        assert_eq!(column(&open, at(3600)), "reviewer 1h, scout 59m");
        assert_eq!(column(&[], at(3600)), "", "a room owing nothing says so");
    }

    #[test]
    fn a_member_who_owes_twice_is_named_once_for_the_older() {
        let open = [member("scout", 300), member("scout", 60)];
        assert_eq!(column(&open, at(360)), "scout 5m");
    }

    #[test]
    fn an_age_reads_the_way_a_person_says_it() {
        assert_eq!(age(at(0), at(0)), "0s");
        assert_eq!(age(at(0), at(59)), "59s");
        assert_eq!(age(at(0), at(60)), "1m");
        assert_eq!(age(at(0), at(7200)), "2h");
        assert_eq!(age(at(60), at(0)), "0s", "a clock that went backwards");
    }

    #[test]
    fn the_room_is_owed_under_its_own_sigil() {
        assert_eq!(column(&[mention(Owed::Room, 0)], at(0)), "@all 0s");
    }

    /// A rostered holder is a debtor like any other, shown by the name
    /// everyone in the room uses for it (ADR-0028).
    #[test]
    fn the_holder_owes_by_the_name_its_members_call_it() {
        let open = [member(crate::name::PARENT, 0)];
        assert_eq!(column(&open, at(60)), "parent 1m");
        assert_eq!(debts("#design", &open)[0].who, "parent");
    }

    /// The whole of what a room's parent is signalled, asserted as one value:
    /// a row per debt, oldest first, and the moment each was asked beside them
    /// rather than in them. This is a payload other processes have already
    /// written and this one still reads, so it is a fixture and not a value
    /// this crate is free to reshape.
    #[test]
    fn the_card_carries_the_table_and_the_debts_it_is_drawn_from() {
        let open = [member("scout", 60), member("reviewer", 0)];
        assert_eq!(
            view(debts("#design", &open)),
            serde_json::json!({
                "kind": "table",
                "headers": ["room", "owed"],
                "rows": [["#design", "reviewer"], ["#design", "scout"]],
                "debts": [
                    {"room": "#design", "who": "reviewer", "at": "1970-01-01T00:00:00Z"},
                    {"room": "#design", "who": "scout", "at": "1970-01-01T00:01:00Z"},
                ],
            }),
            "oldest first, in both halves"
        );

        assert_eq!(
            view(debts("#design", &[])),
            Value::Null,
            "a null payload is what removes the card"
        );
    }

    /// The table rides beside the facts, so a surface that knows only the
    /// vocabulary draws the card and neither half has to know about the other.
    #[test]
    fn the_card_is_both_a_table_and_the_debts_beside_it() {
        let card = view(debts("#design", &[member("reviewer", 0)]));
        let view: View = serde_json::from_value(card).expect("a view a surface can draw");
        assert_eq!(view.fold(), "room · owed\n#design · reviewer");
    }
}
