use super::*;
use crate::views::{Marks, marked_with};
use bingo_sdk::render::Component;
use serde_json::json;

#[test]
fn layout_must_preserve_every_host_cell() {
    let engine = json!({"kind":"Engine","component":"Permission","props":{}});
    let cases = [
        (
            json!({"kind":"Box","borderStyle":"single","children":[engine.clone()]}),
            true,
        ),
        (
            json!({"kind":"Box","height":1,"children":[engine.clone()]}),
            false,
        ),
        (
            json!({"kind":"Box","width":4,"children":[engine.clone()]}),
            false,
        ),
        (
            json!({"kind":"Box","display":"none","children":[engine.clone()]}),
            false,
        ),
        (
            json!({"kind":"Box","width":0,"height":0,"children":[engine.clone()]}),
            false,
        ),
        (
            json!({"kind":"Box","padding":20,"children":[engine.clone()]}),
            false,
        ),
        (
            json!({"kind":"Box","flexDirection":"row","children":[{"kind":"Text","children":["cover"]},engine.clone()]}),
            false,
        ),
        (
            json!({"kind":"Box","height":20,"justifyContent":"flex-end","children":[engine.clone()]}),
            false,
        ),
        (
            json!({"kind":"Box","children":[{"kind":"Text","children":["\u{100000}\u{100001}"]},{"kind":"Box","display":"none","children":[engine]}]}),
            false,
        ),
    ];
    for (value, accepted) in cases {
        let view: View = serde_json::from_value(value).unwrap();
        let expected = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mask = marked_with(
            &view,
            30,
            &Marks {
                sealed: Some(expected.clone()),
                ..Default::default()
            },
            &|component, _, _| {
                assert_eq!(component, Component::Permission);
                vec![
                    Line::from("host input 界"),
                    Line::from(""),
                    Line::from("host choices"),
                ]
                .into()
            },
        );
        assert_eq!(
            whole(&mask.lines, &expected.borrow(), 30, 10).is_some(),
            accepted,
            "{view:?}: {mask:?}"
        );
    }
}

#[test]
fn card_width_height_and_duplicate_cells_fail_closed() {
    let first = char::from_u32(FIRST).unwrap().to_string();
    let second = char::from_u32(FIRST + 1).unwrap().to_string();
    let lines = vec![Line::from(first.repeat(8)), Line::from(second.repeat(8))];
    assert!(whole(&lines, &[8, 8], 8, 2).is_some());
    assert!(whole(&lines, &[8, 8], 7, 2).is_none());
    assert!(whole(&lines, &[8, 8], 8, 1).is_none());
    assert!(whole(&[lines.clone(), lines].concat(), &[8, 8], 8, 4).is_none());
}
