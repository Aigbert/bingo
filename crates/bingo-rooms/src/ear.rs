//! Shared seat and ear vocabulary.
pub use bingo_collab::ear::*;

#[cfg(test)]
mod tests {
use std::time::Duration;
use bingo_sdk::ErrorCode;
use serde_json::{Value, json};
    use super::*;
    use crate::name::PARENT;
    use crate::room;

    fn seat(name: &str, seconds: u64) -> Seat {
        Seat {
            name: name.into(),
            ear: Ear::Patient(Duration::from_secs(seconds)),
        }
    }

    /// ADR-0034 §6: the ear a name alone asks for, which every door falls back
    /// to and every roster is read against.
    #[test]
    fn the_default_ear_is_a_patient_one_at_the_chaser_s_own_patience() {
        assert_eq!(Ear::default(), Ear::Patient(PATIENCE));
        assert_eq!(Ear::default(), Ear::Patient(Duration::from_secs(300)));
        assert_eq!(Seat::named("scout").ear, Ear::default());
    }

    #[test]
    fn a_number_is_the_whole_of_the_dial_and_the_dead_band_is_refused() {
        assert_eq!(Ear::asked(0), Ok(Ear::Live));
        assert_eq!(Ear::asked(30), Ok(Ear::Patient(FLOOR)));
        assert_eq!(Ear::asked(300), Ok(Ear::Patient(PATIENCE)));
        assert_eq!(
            Ear::declared(None),
            Ok(Ear::default()),
            "a name alone takes the default"
        );

        let refused = Ear::asked(15).expect_err("under the floor");
        assert_eq!(refused.code, ErrorCode::InvalidInput);
        assert!(
            refused.message.contains("under thirty seconds of patience"),
            "{refused}"
        );
        assert!(refused.message.contains("live seat"), "{refused}");
        assert!(Ear::asked(29).is_err());
    }

    #[test]
    fn a_journal_is_read_rather_than_refused() {
        assert_eq!(Ear::stored(0), Ear::Live);
        assert_eq!(Ear::stored(15), Ear::Patient(FLOOR), "held at the floor");
        assert_eq!(Ear::stored(120), Ear::Patient(Duration::from_secs(120)));
    }

    #[test]
    fn a_roster_word_says_the_ear_beside_the_name() {
        assert_eq!(Seat::read("scout"), Ok(Seat::named("scout")));
        assert_eq!(Seat::read("parent"), Ok(seat(PARENT, 300)));
        assert_eq!(Seat::read("parent:120"), Ok(seat(PARENT, 120)));
        assert_eq!(Seat::read("parent:0"), Ok(Seat::live(PARENT)));

        let clamped = Seat::read("parent:15").expect_err("the dead band, not a clamp");
        assert!(
            clamped.message.contains("under thirty seconds of patience"),
            "{clamped}"
        );
        assert!(Seat::read("parent:soon").is_err(), "a patience is a number");
    }

    /// A seat reads back as the word that would ask for it, so a roster copied
    /// out of `/room` opens the same room again.
    #[test]
    fn a_seat_reads_back_as_the_word_that_asked_for_it() {
        assert_eq!(Seat::named("scout").said(), "scout");
        assert_eq!(seat(PARENT, 300).said(), "parent");
        assert_eq!(seat(PARENT, 120).said(), "parent:120");
        assert_eq!(Seat::live(PARENT).said(), "parent:0");
        for word in ["scout", "parent:120", "parent:0"] {
            assert_eq!(Seat::read(word).expect("a roster word").said(), word);
        }
    }

    /// The two structured doors declare a listener the same way.
    #[test]
    fn a_listener_is_a_name_or_a_name_and_a_patience() {
        let named: Listener = serde_json::from_value(json!("parent")).expect("a name");
        assert_eq!(named.seat(), Ok(seat(PARENT, 300)));

        let asked: Listener =
            serde_json::from_value(json!({"name": "parent", "patience_s": 120})).expect("a seat");
        assert_eq!(asked.seat(), Ok(seat(PARENT, 120)));

        let live: Listener =
            serde_json::from_value(json!({"name": "parent", "patience_s": 0})).expect("a seat");
        assert_eq!(
            live.seat(),
            Ok(Seat::live(PARENT)),
            "the live seat is asked for"
        );

        let bare: Listener =
            serde_json::from_value(json!({"name": "parent"})).expect("a seat with no number");
        assert_eq!(bare.seat(), Ok(seat(PARENT, 300)));

        let refused: Listener =
            serde_json::from_value(json!({"name": "parent", "patience_s": 15})).expect("a seat");
        assert!(refused.seat().is_err(), "the dead band, at every door");
    }

    #[test]
    fn a_door_s_roster_is_its_members_with_the_listeners_ears() {
        let members = ["scout", "parent"].map(str::to_string).to_vec();
        let listeners = [Listener::Asked {
            name: PARENT.into(),
            patience_s: Some(0),
        }];
        assert_eq!(
            seats(&members, &listeners).expect("a roster"),
            [Seat::named("scout"), Seat::live(PARENT)],
            "a name in both is one seat, wearing the listener's ear"
        );
        assert_eq!(
            seats(&["scout".into()], &listeners).expect("a roster"),
            [Seat::named("scout"), Seat::live(PARENT)],
            "a listener nobody listed is seated by listening"
        );
    }

    /// The whole of the fold, as one table: what the roster declared, what a
    /// seat retuned for itself, and what a reseat leaves standing.
    #[test]
    fn an_ear_is_the_roster_s_until_the_seat_says_otherwise() {
        let mut ears = Ears::default();
        ears.declare(&room::payload(&[Seat::live("scout"), seat(PARENT, 120)]));
        assert_eq!(ears.of("scout"), Ear::Live);
        assert_eq!(ears.of(PARENT), Ear::Patient(Duration::from_secs(120)));
        assert_eq!(ears.of("PARENT"), Ear::Patient(Duration::from_secs(120)));
        assert_eq!(ears.of("nobody"), Ear::default(), "a name off the roster");

        ears.retune("scout", &register(Ear::Patient(FLOOR)));
        assert_eq!(ears.of("scout"), Ear::Patient(FLOOR));
        assert_eq!(ears.retuned(), ["scout"]);

        ears.retune("scout", &Value::Null);
        assert_eq!(ears.of("scout"), Ear::Live, "a cleared retuning");
        assert!(ears.retuned().is_empty());

        ears.declare(&room::payload(&[Seat::named(PARENT)]));
        assert_eq!(
            ears.of(PARENT),
            Ear::default(),
            "a roster is declared whole"
        );
    }

    /// A retuning is written where the roster cannot reach it, so the order the
    /// frames arrive in — a reopened session restates them by kind, not by
    /// journal order — decides nothing.
    #[test]
    fn the_two_layers_are_read_in_their_own_order_whatever_order_they_arrived_in() {
        let roster = room::payload(&[Seat::live(PARENT)]);
        let retuning = register(Ear::Patient(FLOOR));

        let mut first = Ears::default();
        first.declare(&roster);
        first.retune(PARENT, &retuning);

        let mut second = Ears::default();
        second.retune(PARENT, &retuning);
        second.declare(&roster);

        assert_eq!(first, second);
        assert_eq!(first.of(PARENT), Ear::Patient(FLOOR));
    }

    #[test]
    fn a_seat_s_ear_is_published_under_a_kind_of_its_own() {
        assert_eq!(kind("Parent"), "ear:parent");
        assert_eq!(register(Ear::Live), json!({ "patience_s": 0 }));
        assert_eq!(register(Ear::Patient(FLOOR)), json!({ "patience_s": 30 }));
    }
}
