use bingo_sdk::{Next, Tier};

#[test]
fn the_seven_points_use_the_catalogue_names() {
    let points = [
        bingo_sdk::catalogue::PROMPT_SUBMIT,
        bingo_sdk::catalogue::TOOL_CHECK,
        bingo_sdk::catalogue::TOOL_CALL,
        bingo_sdk::catalogue::TURN_STEP,
        bingo_sdk::catalogue::PROMPT_COMPOSE,
        bingo_sdk::catalogue::SESSION_COMPACT,
        bingo_sdk::catalogue::TURN_STOP,
    ];
    assert_eq!(
        serde_json::to_value(points).unwrap(),
        serde_json::json!([
            "prompt.submit",
            "tool.check",
            "tool.call",
            "turn.step",
            "prompt.compose",
            "session.compact",
            "turn.stop"
        ])
    );
    assert_eq!(serde_json::to_value(Tier::default()).unwrap(), "tighten");
}

#[tokio::test]
async fn a_continuation_passes_its_input_once() {
    let next = Next::new(|n: u32| Box::pin(async move { Ok(n + 1) }));
    assert_eq!(next.run(4).await.unwrap(), 5);
}
