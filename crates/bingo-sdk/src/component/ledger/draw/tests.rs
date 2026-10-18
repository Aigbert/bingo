use super::*;

fn step(state: StepState) -> Step {
    Step {
        mark: '¶',
        verb: "read".into(),
        target: "src/main.rs".into(),
        reads: Some(Reads::Path),
        tier: Tier::Lookup,
        state,
        upshot: None,
        answer: None,
        elapsed_ms: Some(1234),
        why: None,
        landed_at: Some(100),
    }
}
fn marks() -> Marks {
    Marks {
        asked: "◆".into()
    }
}

#[test]
fn step_rows_keep_the_grid_and_right_margin() {
    let mut step = step(StepState::Done);
    step.upshot = Some(Upshot::plain("214 lines"));
    let text = step.drawing(&marks(), 60).fold();
    assert_eq!(text.chars().nth(3), Some('¶'));
    assert_eq!(&text[6..10], "read");
    assert_eq!(text.find("src/"), Some(14)); // UTF-8 mark uses two bytes.
    assert_eq!(text.width(), 58);
    assert!(text.ends_with("214 lines"));
    step.verb = "investigate".into();
    assert_eq!(step.target_column(), 18);
}

#[test]
fn step_rows_say_their_state_at_the_right() {
    for (state, right, ink, motion) in [
        (StepState::Running, "1s", "glow", "spin"),
        (StepState::Asked, "needs you", "amberGlow", "breathe"),
        (StepState::Done, "", "muted", "fade"),
        (StepState::Failed, "failed", "bad", "fade"),
        (StepState::Stopped, "stopped", "faint", "fade"),
        (StepState::Skipped, "said no", "faint", "fade"),
    ] {
        let view = step(state).drawing(&marks(), 60);
        assert!(view.fold().ends_with(right));
        let value = serde_json::to_value(view).unwrap();
        assert_eq!(value["children"][1]["color"], ink);
        assert_eq!(value["children"][1]["motion"]["kind"], motion);
    }
}

#[test]
fn targets_keep_their_tier_command_and_path_inks() {
    let mut step = step(StepState::Done);
    assert_eq!(
        step.target_segments(30, false),
        [("src/".into(), "dim"), ("main.rs".into(), "muted")]
    );
    step.tier = Tier::Change;
    step.reads = Some(Reads::Command);
    assert_eq!(
        step.target_segments(30, false),
        [("src/main.rs".into(), "code")]
    );
    step.state = StepState::Stopped;
    assert_eq!(
        step.target_segments(30, false),
        [("src/main.rs".into(), "dim")]
    );
    assert!(
        step.target_segments(6, true)
            .iter()
            .map(|(s, _)| s.as_str())
            .collect::<String>()
            .ends_with("...")
    );
}

#[test]
fn a_failed_turn_says_failed_once_on_the_step_that_failed() {
    assert_eq!(
        step(StepState::Failed)
            .drawing(&marks(), 60)
            .fold()
            .matches("failed")
            .count(),
        1
    );
}

#[test]
fn an_answered_step_keeps_its_outcome_and_answer_at_the_right() {
    let mut step = step(StepState::Done);
    step.upshot = Some(Upshot::Diff {
        added: 1,
        removed: 2,
    });
    step.answer = Some(Upshot::plain("changed by you"));
    assert!(
        step.drawing(&marks(), 80)
            .fold()
            .ends_with("+1 −2 · changed by you")
    );
    step.upshot = None;
    step.answer = Some(Upshot::plain("Alpha, Beta"));
    assert!(step.drawing(&marks(), 80).fold().ends_with("  Alpha, Beta"));
    assert!(!step.drawing(&marks(), 80).fold().contains('·'));
}

#[test]
fn outcomes_answers_and_reasons_keep_their_order_without_a_leading_separator() {
    let mut value = step(StepState::Done);
    value.upshot = Some(Upshot::plain("3 passed"));
    value.answer = Some(Upshot::plain("changed by you"));
    let mut json = serde_json::to_value(value).unwrap();
    json["why"] = serde_json::json!("accept edits");
    let mut value: Step = serde_json::from_value(json).unwrap();
    assert!(
        value
            .drawing(&marks(), 100)
            .fold()
            .ends_with("3 passed · changed by you · accept edits")
    );
    value.upshot = None;
    value.answer = None;
    let row = value.drawing(&marks(), 80).fold();
    assert!(row.ends_with("accept edits"));
    assert!(!row.contains('·'));
}
