use bingo_sdk::slot::Place;
#[test]
fn dock_placement_round_trips_without_changing_floating_anchors() {
    let place: Place = serde_json::from_str("\"dock\"").unwrap();
    assert_eq!(place, Place::Dock);
    assert_eq!(serde_json::to_string(&place).unwrap(), "\"dock\"");
    assert!(place.published_ok());
    let rect = serde_json::json!({"rect":{"anchor":"bottomLeft","width":"100%","height":18}});
    let place: Place = serde_json::from_value(rect.clone()).unwrap();
    assert_eq!(serde_json::to_value(place).unwrap(), rect);
}
