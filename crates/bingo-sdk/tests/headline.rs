use bingo_sdk::{Headline, Input, Item, Tone};

fn headline() -> Headline {
    Headline {
        tone: Tone::Good,
        parts: vec![
            "`echo ok` finished in the background".into(),
            "exit 0".into(),
        ],
    }
}

#[test]
fn headlined_and_plain_inputs_round_trip_byte_identically() {
    for (wire, expected) in [
        (
            include_str!("../fixtures/headline/input.json"),
            Some(headline()),
        ),
        (include_str!("../fixtures/headline/plain.json"), None),
    ] {
        let input: Input = serde_json::from_str(wire).unwrap();
        assert!(matches!(&input, Input::Text { headline, .. } if headline == &expected));
        assert_eq!(serde_json::to_string(&input).unwrap(), wire.trim_end());
    }
}

#[test]
fn a_headline_lives_in_item_meta_and_round_trips_through_accessors() {
    let wire = include_str!("../fixtures/headline/item.json");
    let mut item: Item = serde_json::from_str(wire).unwrap();
    assert_eq!(item.headline(), Some(headline()));
    assert_eq!(
        serde_json::to_value(&item).unwrap(),
        serde_json::from_str::<serde_json::Value>(wire).unwrap()
    );
    item.meta.clear();
    assert_eq!(item.headline(), None);
    item.set_headline(headline());
    assert_eq!(item.headline(), Some(headline()));
    assert_eq!(item.meta.len(), 1);
}
