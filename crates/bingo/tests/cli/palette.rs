use super::*;
use bingo_sdk::IntentOutcome;

#[test]
fn palette_commands_return_client_actions_as_clean_ndjson_without_a_turn() {
    let home = tempfile::tempdir().unwrap();
    let script = script(r#"{"responses":[]}"#);
    for (command, group) in [("/help", "keys"), ("/resume", "sessions")] {
        let out = run_within(
            bingo()
                .env("BINGO_FAKE_SCRIPT", script.path())
                .envs(home_env(home.path()))
                .args([
                    "--print",
                    "--provider",
                    "fake",
                    "--output-format",
                    "json",
                    "--cwd",
                ])
                .arg(home.path())
                .arg(command),
            PATIENCE,
        );
        assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
        let frames = frames_of(&out);
        assert!(
            !frames
                .iter()
                .any(|frame| matches!(frame.event, Event::TurnStarted { .. }))
        );
        assert!(frames.iter().any(|frame| matches!(&frame.event,
            Event::IntentAck { outcome: IntentOutcome::Applied { result }, .. }
                if result["surface"]["name"] == "Palette" && result["surface"]["args"]["open"] == group)));
    }
    let out = run_within(
        bingo()
            .env("BINGO_FAKE_SCRIPT", script.path())
            .envs(home_env(home.path()))
            .args(["--print", "--provider", "fake", "--cwd"])
            .arg(home.path())
            .arg("/help"),
        PATIENCE,
    );
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(stdout(&out).is_empty(), "{}", stdout(&out));
}
