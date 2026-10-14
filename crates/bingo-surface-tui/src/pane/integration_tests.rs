use crate::test_support::*;
use bingo_sdk::slot::Slot;
use serde_json::json;

fn state_with(place: serde_json::Value) -> bingo_sdk::SessionState {
    let mut state = state();
    let slot: Slot = serde_json::from_value(json!({"kind":"pane","id":"counter","title":"Counter",
        "focus":true,"closeOnEscape":true,"holdToasts":false,"asked":true,
        "place":place,"size":{"rows":{"min":8,"ideal":10,"max":20},
        "columns":{"min":48,"ideal":64,"max":80}},
        "view":{"request":{"component":"Pane","requestId":"counter","surface":"terminal",
        "props":{},"viewport":{"columns":0,"rows":0}}}})).unwrap();
    state.signals.entry("counter".into()).or_default().insert("pane".into(),
        bingo_sdk::slot::with(json!({}),[slot]));
    state
}
#[test]
fn seated_pulls_change_place_width_and_height_and_collapse_without_a_request() {
    let state=state_with(json!(["side","above"]));
    let (mut ui,now)=scene();
    for (width,height,place,columns,rows) in [(200,50,"side",64,43),(100,30,"above",100,9)] {
        draw_sized(width,height,&state,&mut ui,now);
        let viewport=serde_json::to_value(&ui.shown.pulls[0].request.viewport).unwrap();
        assert_eq!(viewport,json!({"place":place,"columns":columns,"rows":rows}));
    }
    let screen=draw_sized(60,15,&state,&mut ui,now);
    assert!(ui.shown.pulls.is_empty());
    assert!(screen.contains("Counter ▸"),"{screen}");
}
