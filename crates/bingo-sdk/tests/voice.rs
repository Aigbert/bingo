#![allow(clippy::unwrap_used, clippy::expect_used)]
use bingo_sdk::{ItemBody, Origin, Voice};
use serde_json::{Value, json};

#[test]
fn voice_round_trips_the_golden_json() {
    let voices = [Voice::Said, Voice::Note, Voice::Quiet, Voice::Aside];
    let golden: Value = serde_json::from_str(include_str!("../fixtures/voice/all.json")).unwrap();
    assert_eq!(serde_json::to_value(voices).unwrap(), golden);
    assert_eq!(serde_json::from_value::<Vec<Voice>>(golden).unwrap(), voices);
    let origin = Origin::surface("stranger").voiced(Voice::Aside);
    assert_eq!(serde_json::to_value(&origin).unwrap(), json!({"surface": "stranger", "voice": "aside"}));
    assert_eq!(serde_json::from_value::<Origin>(serde_json::to_value(&origin).unwrap()).unwrap(), origin);
}

#[test]
fn voice_absence_keeps_the_original_record_readable_and_unchanged() {
    let golden: Value = serde_json::from_str(include_str!("../fixtures/record/item/user.json")).unwrap();
    let body: ItemBody = serde_json::from_value(golden.clone()).unwrap();
    let ItemBody::User { origin, .. } = &body else { panic!("user fixture") };
    assert_eq!(origin.voice, None);
    assert_eq!(serde_json::to_value(body).unwrap(), golden);
}
