//! A step that holds more than one call: both come back, in landing order.

use super::*;

/// Two commands asked for in one step both come back on one round of one turn.
#[test]
fn two_commands_in_one_step_both_come_back() {
    let home = tempfile::tempdir().unwrap();
    let script = script(
        r#"{"responses":[
            {"steps":[
                {"toolCall":{"name":"Bash","input":{"command":"echo first"}}},
                {"toolCall":{"name":"Bash","input":{"command":"echo second"}}}
            ]},
            {"steps":[{"text":"Both ran."}]}
        ]}"#,
    );
    let out = scripted_run(
        home.path(),
        &script,
        &["--dangerously-skip-permissions"],
        "run both",
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let frames = frames_of(&out);
    assert!(
        matches!(
            frames.last().map(|f| &f.event),
            Some(Event::TurnCompleted {
                status: TurnStatus::Completed,
                ..
            })
        ),
        "{}",
        stdout(&out)
    );
    assert_both_returned(&frames);
    let reversed: Vec<_> = frames.into_iter().rev().collect();
    assert_both_returned(&reversed);
}

fn assert_both_returned(frames: &[bingo_sdk::Frame]) {
    let calls: Vec<(u32, &serde_json::Value, String)> = frames
        .iter()
        .filter_map(|f| match &f.event {
            Event::ItemCompleted { item } => match &item.body {
                bingo_sdk::ItemBody::ToolCall {
                    name,
                    input,
                    output: Some(output),
                    ..
                } if name == "Bash" => Some((
                    item.round,
                    &input["command"],
                    output
                        .parts
                        .iter()
                        .filter_map(bingo_sdk::ContentPart::as_text)
                        .collect(),
                )),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(calls.len(), 2, "both calls landed: {frames:?}");
    assert_eq!(calls[0].0, calls[1].0, "one step, not two rounds");
    for (command, expected) in [("echo first", "first"), ("echo second", "second")] {
        let (_, _, output) = calls
            .iter()
            .find(|(_, input, _)| *input == command)
            .unwrap();
        assert!(output.contains(expected), "{calls:?}");
    }
}
