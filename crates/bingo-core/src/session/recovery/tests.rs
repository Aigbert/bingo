use super::*;

#[test]
fn unfinished_items_close_at_the_given_instant() {
    let turn = TurnId::from_raw("lost");
    let open = [
        crate::turn::mint(
            &turn,
            0,
            ItemBody::ToolCall {
                call_id: "c0".into(),
                name: "Echo".into(),
                input: serde_json::json!({}),
                output: None,
                duration_ms: None,
                progress: None,
                allowed_by: None,
                asked_at: None,
                began_at: None,
                quiet: false,
            },
            ItemStatus::Pending,
        ),
        crate::turn::mint(
            &turn,
            0,
            ItemBody::Assistant {
                text: "unfinished".into(),
            },
            ItemStatus::Running,
        ),
    ];
    let at = "2026-10-19T00:00:00Z".parse().unwrap();
    let closed = interrupt_items(open.iter(), at);
    let expected: Vec<_> = open
        .into_iter()
        .map(|mut item| {
            item.status = ItemStatus::Interrupted;
            item.completed_at = Some(at);
            Event::ItemCompleted { item }
        })
        .collect();
    assert_eq!(closed, expected);
}
