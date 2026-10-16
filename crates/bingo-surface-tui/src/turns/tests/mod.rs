use super::*;
use crate::test_support::{finished_turn, folded, running_turn, stopped_turn, tool, worked};

fn roles(state: &SessionState, folds: &Folds) -> Roles {
    Roles::of(state, &Agents::new(), folds, &Answers::of(state))
}

fn id(raw: &str) -> ItemId {
    ItemId::from_raw(raw)
}

fn verbs(steps: &[Step]) -> Vec<&str> {
    steps.iter().map(|step| step.verb.as_str()).collect()
}

#[test]
fn a_running_turn_gives_each_step_a_row_and_no_thought_a_row() {
    let state = folded(running_turn());
    let roles = roles(&state, &Folds::new());
    assert_eq!(roles.of_item(&id("trn_1_1")), &Role::Hidden, "the thought");
    let Role::Step { step, opened } = roles.of_item(&id("trn_1_2")) else {
        panic!("the read is a step: {:?}", roles.of_item(&id("trn_1_2")));
    };
    assert_eq!((step.verb.as_str(), step.state), ("read", StepState::Done));
    assert!(!opened);
    assert_eq!(
        step.target, "src/main.rs",
        "read from the session's directory"
    );
    assert!(matches!(
        roles.of_item(&id("trn_1_6")),
        Role::Step { step, .. } if step.state == StepState::Running
    ));
    assert_eq!(
        roles.of_item(&id("trn_1_4")),
        &Role::Own,
        "words stay words"
    );
    assert!(roles.spinning());
}

#[test]
fn a_finished_turn_folds_onto_its_first_step_and_keeps_its_answer() {
    let state = folded(finished_turn());
    let roles = roles(&state, &Folds::new());
    let Role::Fold { fold, then } = roles.of_item(&id("trn_1_1")) else {
        panic!("the first of the working holds the line");
    };
    assert_eq!(verbs(&fold.steps), ["read", "grep", "edit", "bash"]);
    assert_eq!(fold.outcome, TurnEnd::Done);
    assert!(fold.thought && !fold.is_expanded && then.is_none());
    assert_eq!(fold.elapsed, 8_000, "first start to last end");
    for hidden in ["trn_1_2", "trn_1_3", "trn_1_4", "trn_1_5", "trn_1_6"] {
        assert_eq!(roles.of_item(&id(hidden)), &Role::Hidden, "{hidden}");
    }
    assert_eq!(roles.of_item(&id("trn_1_7")), &Role::Own, "the answer");
    assert_eq!(roles.of_item(&id("trn_1_0")), &Role::Own, "the prompt");
    assert!(!roles.spinning());
}

#[test]
fn an_opened_turn_shows_its_steps_thoughts_and_words_under_its_line() {
    let state = folded(finished_turn());
    let mut folds = Folds::new();
    folds.insert(id("trn_1_1"), Fold::Open);
    let roles = roles(&state, &folds);
    let Role::Fold { fold, then } = roles.of_item(&id("trn_1_1")) else {
        panic!("the line stays where it was");
    };
    assert!(fold.is_expanded);
    assert_eq!(
        then.as_deref(),
        Some(&Role::Thought),
        "the thought under it"
    );
    assert!(matches!(
        roles.of_item(&id("trn_1_2")),
        Role::Step { opened: true, .. }
    ));
    assert!(matches!(
        roles.of_item(&id("trn_1_5")),
        Role::Step { opened: true, step } if step.upshot == Some(Upshot::Diff { added: 1, removed: 1 })
    ));
    assert_eq!(
        roles.of_item(&id("trn_1_4")),
        &Role::Own,
        "the word between"
    );
}

#[test]
fn a_stopped_turn_says_how_many_of_its_steps_had_run() {
    let state = folded(stopped_turn());
    let roles = roles(&state, &Folds::new());
    assert_eq!(
        roles.of_item(&id("trn_1_8")),
        &Role::Stop { ran: 3, total: 4 }
    );
    let Role::Fold { fold, .. } = roles.of_item(&id("trn_1_1")) else {
        panic!("a stopped turn folds too");
    };
    assert_eq!(fold.outcome, TurnEnd::Stopped);
    assert_eq!(fold.steps[3].state, StepState::Stopped);
}

/// A step the person said no to came back, but it did not run.
#[test]
fn a_step_the_person_refused_is_not_counted_as_run() {
    let mut frames = stopped_turn();
    let refusal = Item {
        turn: Some(bingo_sdk::TurnId::from_raw("trn_1")),
        ..crate::test_support::item(
            "trn_1_r",
            ItemStatus::Completed,
            ItemBody::PermissionReceipt {
                item: Some(id("trn_1_5")),
                interaction: bingo_sdk::InteractionId::from_raw("int_1"),
                tool: "Edit".into(),
                decision: DecisionKind::Deny,
                feedback: Some("not that file".into()),
            },
        )
    };
    frames.push(crate::test_support::frame(
        99,
        bingo_sdk::Event::ItemCompleted { item: refusal },
    ));
    let state = folded(frames);
    assert_eq!(
        roles(&state, &Folds::new()).of_item(&id("trn_1_8")),
        &Role::Stop { ran: 2, total: 4 }
    );
}

#[test]
fn every_part_of_a_turn_opens_from_the_line_that_holds_it() {
    let state = folded(finished_turn());
    let shut = roles(&state, &Folds::new());
    assert_eq!(shut.line_of(&state, &id("trn_1_1")), Some(id("trn_1_1")));
    assert_eq!(shut.line_of(&state, &id("trn_1_5")), Some(id("trn_1_1")));
    assert_eq!(shut.line_of(&state, &id("trn_1_7")), None, "the answer");
    let mut folds = Folds::new();
    folds.insert(id("trn_1_1"), Fold::Open);
    let opened = roles(&state, &folds);
    assert_eq!(opened.line_of(&state, &id("trn_1_6")), Some(id("trn_1_1")));
    assert_eq!(opened.line_of(&state, &id("trn_1_2")), Some(id("trn_1_1")));
}

#[test]
fn a_long_run_of_one_verb_folds_to_a_count() {
    let mut items = worked("trn_1");
    let reads = (0..18).map(|n| {
        let input = serde_json::json!({ "file_path": format!("/tmp/project/src/{n}.rs") });
        tool(
            &format!("rd_{n}"),
            "Read",
            input,
            None,
            ItemStatus::Completed,
        )
    });
    items.splice(2..3, reads);
    let frames = items
        .into_iter()
        .zip(1..)
        .map(|(item, seq)| {
            crate::test_support::frame(seq, bingo_sdk::Event::ItemCompleted { item })
        })
        .collect();
    let state = folded(frames);
    let roles = roles(&state, &Folds::new());
    let Role::Fold { fold, .. } = roles.of_item(&id("trn_1_1")) else {
        panic!("the thought before the reads holds the line");
    };
    let line: String = fold
        .row(120, |s| s.chars().count())
        .unwrap_or_default()
        .into_iter()
        .map(|piece| piece.text)
        .collect();
    assert!(
        line.starts_with("   ▸ 21 steps · read ×18 · grep · edit +1 −1 · bash 3 passed"),
        "{line:?}"
    );
}

#[test]
fn a_call_is_named_by_its_tool_in_plain_words() {
    assert_eq!(verb("Read"), "read");
    assert_eq!(verb("WebSearch"), "web search");
    assert_eq!(verb("mcp__x"), "mcp__x");
}

/// The finished turn with the edit answered `decision`, as its fold says it.
fn folded_with(decision: DecisionKind) -> TurnFoldProps {
    let mut items = worked("trn_1");
    let receipt = crate::test_support::item(
        "trn_1_r",
        ItemStatus::Completed,
        ItemBody::PermissionReceipt {
            item: Some(id("trn_1_5")),
            interaction: bingo_sdk::InteractionId::from_raw("int_1"),
            tool: "Edit".into(),
            decision,
            feedback: None,
        },
    );
    items.insert(
        6,
        Item {
            turn: Some(bingo_sdk::TurnId::from_raw("trn_1")),
            ..receipt
        },
    );
    let frames = items
        .into_iter()
        .zip(1..)
        .map(|(item, seq)| {
            crate::test_support::frame(seq, bingo_sdk::Event::ItemCompleted { item })
        })
        .collect();
    let state = folded(frames);
    match roles(&state, &Folds::new()).of_item(&id("trn_1_1")) {
        Role::Fold { fold, .. } => fold.clone(),
        other => panic!("the turn folds: {other:?}"),
    }
}

#[test]
fn a_plain_yes_leaves_the_fold_and_a_changed_one_stays() {
    let answers = |fold: &TurnFoldProps| -> Vec<Option<Upshot>> {
        fold.steps.iter().map(|step| step.answer.clone()).collect()
    };
    assert!(
        answers(&folded_with(DecisionKind::Allow))
            .iter()
            .all(Option::is_none),
        "a yes is how the work went on"
    );
    let changed = folded_with(DecisionKind::AllowChanged);
    assert!(
        answers(&changed)
            .iter()
            .flatten()
            .any(|answer| matches!(answer, Upshot::Words { text, .. } if text.contains("changed"))),
        "{changed:?}"
    );
}
