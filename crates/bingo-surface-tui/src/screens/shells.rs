//! The screens a shell running in the background is read through (M75):
//! Claude Code's own shape, read off its screen and drawn in bingo's grammar.
//!
//! Three things are being decided here and nowhere else: that the row which
//! started a shell says where it went and, once it has gone, that it went;
//! that the status line counts the shells still running; and that the running
//! set stays data for the placed row.

use super::*;

const ID: &str = "job_ab12cd34";
const COMMAND: &str = "cargo test --workspace";

/// The set as the shell's plugin signals it while the one shell runs.
fn set() -> serde_json::Value {
    crate::test_lanes::shell_payload(
        json!({
            "kind": "table",
            "headers": ["job", "command", "since"],
            "rows": [[ID, COMMAND, "10:22:07"]],
        }),
        "itm_2",
    )
}

/// The call that went to the background, already come back: the text the
/// model read, and beside it the kind the plugin named (ADR-0038).
fn backgrounded(seq: u64) -> bingo_sdk::Frame {
    let output = ToolOutput {
        parts: vec![ContentPart::text(format!(
            "Started `{COMMAND}` in the background as job {ID}. You will be told when it ends."
        ))],
        is_error: false,
        display: Some(View::Custom {
            kind: "job".into(),
            data: json!({ "id": ID, "command": COMMAND }),
            fold: format!("Started in the background as {ID}"),
        }),
        upshot: Some(bingo_sdk::Upshot::plain("in background")),
    };
    item(
        seq,
        tool(
            "itm_2",
            "Bash",
            json!({ "command": COMMAND, "background": true }),
            Some(output),
            ItemStatus::Completed,
        ),
    )
}

/// The shell still running: the set signalled, the model's word said.
fn running() -> bingo_sdk::SessionState {
    folded(vec![
        item(1, user("itm_1", "run the tests in the background")),
        backgrounded(2),
        frame(3, signalled("bingo.tools.bash", "jobs", set())),
        item(4, assistant("itm_3", "Started.", ItemStatus::Completed)),
    ])
}

/// The same session after the shell ended: the set taken away, and the
/// completion delivered as the turn it opens.
fn ran() -> bingo_sdk::SessionState {
    folded(vec![
        item(1, user("itm_1", "run the tests in the background")),
        backgrounded(2),
        frame(3, signalled("bingo.tools.bash", "jobs", set())),
        item(4, assistant("itm_3", "Started.", ItemStatus::Completed)),
        frame(
            5,
            signalled("bingo.tools.bash", "jobs", serde_json::Value::Null),
        ),
        item(
            6,
            super::job_notice(
                "itm_4",
                COMMAND,
                &format!(
                    "Background job exited with code 0.\n\
                     `BashOutput` with id \"{ID}\" reads what it wrote."
                ),
            ),
        ),
        item(
            7,
            assistant("itm_5", "The tests passed.", ItemStatus::Completed),
        ),
    ])
}

/// `⎿  Running in the background` under the call, `1 shell` on the status
/// line, without a second card.
#[test]
fn shell_running() {
    let (mut ui, now) = scene();
    let state = running();
    let narrow = draw_tree(80, 24, &solo(&state), &mut ui, now);
    assert!(narrow.contains("in background"), "{narrow}");
    assert!(!narrow.contains("⎿"), "{narrow}");
    assert!(narrow.contains("1 shell"), "{narrow}");
    assert!(!narrow.contains("job_ab12cd34"), "{narrow}");
    let wide = draw_tree(120, 40, &solo(&state), &mut ui, now);
    assert!(!wide.contains("since"), "no card: {wide}");
    both("shell_running", &solo(&state), &mut ui, now);
}

/// The same row reads the display fold once the set no longer lists
/// the shell, the count is gone, and the completion is a turn of its own.
#[test]
fn shell_ran() {
    let (mut ui, now) = scene();
    let state = ran();
    let shut = draw_tree(80, 24, &solo(&state), &mut ui, now);
    assert!(
        shut.contains("   $ run     cargo test --workspace"),
        "{shut}"
    );
    open_the_turns(&mut ui, &state);
    let screen = draw_tree(80, 24, &solo(&state), &mut ui, now);
    assert!(
        screen.contains("     Started in the background as job_ab12cd34"),
        "under the step's words: {screen}"
    );
    assert!(!screen.contains("Running in the background"), "{screen}");
    assert!(!screen.contains("1 shell"), "{screen}");
    assert!(screen.contains("finished in the background"), "{screen}");
    both("shell_ran", &solo(&state), &mut ui, now);
}

#[test]
fn shell_and_wake_follow_plugin_order() {
    let (mut ui, now) = scene();
    let mut state = running();
    let at = now.wall + jiff::SignedDuration::from_secs(240);
    state.apply(&frame(state.seq.0 + 1, pending_wake(&at.to_string())));
    let screen = draw_tree(120, 40, &solo(&state), &mut ui, now);
    assert!(screen.find("wake in 4m").unwrap() < screen.find("1 shell").unwrap());
    both("shell_and_wake", &solo(&state), &mut ui, now);
}
