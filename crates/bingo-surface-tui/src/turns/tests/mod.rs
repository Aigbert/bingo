use super::*;
use crate::test_support::{finished_turn, folded, running_turn, stopped_turn, tool};
fn id(raw: &str) -> ItemId {
    ItemId::from_raw(raw)
}

#[test]
fn running_finished_and_stopped_turns_keep_each_step_row() {
    for frames in [running_turn(), finished_turn(), stopped_turn()] {
        let state = folded(frames);
        let folds = Default::default();
        let pictures = Default::default();
        let linked = Default::default();
        let rows = crate::transcript::Rows::of(
            &state,
            80,
            &folds,
            &pictures,
            &linked,
            crate::test_support::scene().1,
        );
        let roles = Roles::of(&Agents::new(), &rows);
        for item in state.transcript.items() {
            match &item.body {
                ItemBody::ToolCall { .. } => {
                    assert!(matches!(roles.of_item(&item.id), Role::Step { .. }))
                }
                ItemBody::Reasoning { .. } | ItemBody::Interruption { .. } => {
                    assert_eq!(roles.of_item(&item.id), &Role::Hidden)
                }
                _ => {}
            }
        }
    }
}

#[test]
fn a_step_reads_the_catalogue_and_uses_only_trusted_baseline_traits() {
    use bingo_sdk::component::Tier;
    use serde_json::json;
    let state = folded(running_turn());
    let item = state
        .transcript
        .items()
        .iter()
        .find(|item| item.id == id("trn_1_2"))
        .unwrap();
    let mut entry = bingo_sdk::CatalogEntry {
        id: "Read".into(),
        label: "Read".into(),
        meta: json!({"look":{"mark":"¶","ascii":"<","verb":"inspect",
            "target":{"field":"file_path","reads":"path"}},
            "traits":bingo_sdk::ToolTraits::read_only()}),
    };
    let resolved = step(item, &state, &crate::tool_rows::Looks::of(&[entry.clone()]));
    assert_eq!(resolved.verb, "inspect");
    assert_eq!(resolved.reads, Some(bingo_sdk::Reads::Path));
    assert_eq!(resolved.tier, Tier::Lookup);
    assert_eq!(resolved.target, "src/main.rs");
    for traits in [
        json!({"trusted":false,"readOnly":true}),
        json!({"trusted":true,"readOnly":false}),
        json!({}),
        json!(null),
    ] {
        entry.meta["traits"] = traits;
        assert_eq!(
            step(item, &state, &crate::tool_rows::Looks::of(&[entry.clone()])).tier,
            Tier::Change
        );
    }
    let unknown = step(item, &state, &Default::default());
    assert_eq!(unknown.tier, Tier::Change);
    assert_eq!(unknown.verb, "read");
    assert_eq!(unknown.reads, Some(bingo_sdk::Reads::Text));
}

#[test]
fn transcript_steps_keep_declared_targets() {
    let state = folded(running_turn());
    for (name, input, look, _, target) in crate::tool_rows::cases() {
        let tools = crate::tool_rows::declared(name, look.clone());
        let item = tool("itm_look", name, input, None, ItemStatus::Completed);
        let step = step(&item, &state, &tools);
        assert_eq!(step.target, target);
        assert_eq!(step.verb, look.verb);
    }
}

#[test]
fn a_non_call_step_resolves_the_ascii_fallback() {
    let state = folded(running_turn());
    let item = state
        .transcript
        .items()
        .iter()
        .find(|i| !matches!(i.body, ItemBody::ToolCall { .. }))
        .unwrap();
    let theme = crate::theme::Theme {
        colors: crate::theme::Colors::Ansi,
        glyphs: &crate::theme::ASCII,
    };
    crate::theme::with(theme, || {
        assert_eq!(step(item, &state, &Default::default()).mark, '-');
    });
}

#[test]
fn question_results_are_answers_without_a_speaker() {
    for (input, receipt, expected) in [
        (
            serde_json::json!({"questions":[{"header":"Route"}]}),
            "Route: Beta",
            "Beta",
        ),
        (
            serde_json::json!({"questions":[{"header":"Route"}]}),
            "Route: Alpha, Beta",
            "Alpha, Beta",
        ),
        (
            serde_json::json!({"questions":[{}, {}, {}]}),
            "A: one; B: two; C: three",
            "3 answers",
        ),
    ] {
        assert_eq!(question_result(receipt, &input), expected);
    }
}

#[test]
fn a_refused_step_is_skipped_and_a_changed_permission_stays() {
    use crate::test_support::*;
    for (decision, expected, answer) in [
        (DecisionKind::Deny, StepState::Skipped, None),
        (DecisionKind::Allow, StepState::Done, None),
        (
            DecisionKind::AllowChanged,
            StepState::Done,
            Some(Upshot::plain("changed by you")),
        ),
    ] {
        let mut state = folded(finished_turn());
        let receipt = item(
            "receipt",
            ItemStatus::Completed,
            ItemBody::PermissionReceipt {
                asked_at: None,
                item: Some(id("trn_1_5")),
                interaction: bingo_sdk::InteractionId::from_raw("int_1"),
                tool: "Edit".into(),
                decision,
                feedback: None,
            },
        );
        state.apply(&frame(
            99,
            bingo_sdk::Event::ItemCompleted { item: receipt },
        ));
        let call = state.transcript.get(&id("trn_1_5")).unwrap();
        let row = step(call, &state, &Default::default());
        assert_eq!(row.state, expected);
        assert_eq!(row.answer, answer);
    }
}

#[test]
fn only_person_visible_policy_reasons_appear_on_steps() {
    use crate::test_support::*;
    use bingo_sdk::{Reason, policy::RuleSource};
    for (reason, expected) in [
        (
            Reason::Mode {
                mode: "acceptEdits".into(),
            },
            Some("accept edits"),
        ),
        (
            Reason::Mode {
                mode: "bypassPermissions".into(),
            },
            Some("bypass"),
        ),
        (
            Reason::Rule {
                rule: "inspect".into(),
                source: Some(RuleSource::Session),
            },
            Some("session rule"),
        ),
        (
            Reason::Rule {
                rule: "inspect".into(),
                source: Some(RuleSource::Settings),
            },
            None,
        ),
        (
            Reason::Rule {
                rule: "inspect".into(),
                source: None,
            },
            None,
        ),
        (
            Reason::Hook {
                hook: "gate".into(),
            },
            None,
        ),
        (Reason::ReadOnly, None),
        (Reason::Default, None),
    ] {
        let state = with_permission_mode("acceptEdits");
        let mut call = tool(
            "call",
            "Inspect",
            serde_json::json!({}),
            None,
            ItemStatus::Completed,
        );
        if let ItemBody::ToolCall { allowed_by, .. } = &mut call.body {
            *allowed_by = Some(Box::new(reason));
        }
        assert_eq!(
            step(&call, &state, &Default::default()).why,
            expected.map(str::to_owned)
        );
    }
}

#[test]
fn a_running_steps_clock_starts_at_its_permission_receipt() {
    use crate::test_support::*;
    let call = tool(
        "call",
        "Inspect",
        serde_json::json!({}),
        None,
        ItemStatus::Running,
    );
    let mut receipt = item(
        "receipt",
        ItemStatus::Completed,
        ItemBody::PermissionReceipt {
            item: Some(call.id.clone()),
            interaction: bingo_sdk::InteractionId::from_raw("ask"),
            tool: "Inspect".into(),
            decision: DecisionKind::Allow,
            feedback: None,
            asked_at: Some(ts()),
        },
    );
    receipt.started_at = ts() + jiff::SignedDuration::from_millis(4123);
    receipt.completed_at = Some(receipt.started_at);
    let state = folded(vec![
        frame(1, started("trn_1")),
        frame(2, bingo_sdk::Event::ItemStarted { item: call.clone() }),
        frame(3, bingo_sdk::Event::ItemCompleted { item: receipt }),
    ]);
    let now = still(later(scene().1, 5122));
    let answers = Answers::of(&state);
    assert_eq!(
        running_for(&call, &answers, later(scene().1, 4122)),
        jiff::SignedDuration::ZERO
    );
    assert_eq!(running_for(&call, &answers, now).as_millis(), 999);
    let step = step(&call, &state, &Default::default());
    let (_, Props::Ledger(props)) = ledger(&call, &step, &answers, now) else {
        panic!("ledger");
    };
    assert_eq!(props.steps[0].elapsed_ms, Some(0));
    assert_eq!(
        bingo_sdk::transcript::next_second(now.wall, running_for(&call, &answers, now)),
        Some(ts() + jiff::SignedDuration::from_millis(5123))
    );
}

fn step(item: &Item, state: &SessionState, tools: &crate::tool_rows::Looks) -> Step {
    let folds = Default::default();
    let pictures = Default::default();
    let linked = Default::default();
    let mut rows = crate::transcript::Rows::of(
        state,
        80,
        &folds,
        &pictures,
        &linked,
        crate::test_support::scene().1,
    );
    rows.tools = tools.clone();
    super::step(item, &rows)
}

#[test]
fn a_changed_call_keeps_both_the_answer_and_the_policy_reason() {
    use crate::test_support::*;
    let mut call = tool(
        "call",
        "Inspect",
        serde_json::json!({}),
        None,
        ItemStatus::Completed,
    );
    if let ItemBody::ToolCall { allowed_by, .. } = &mut call.body {
        *allowed_by = Some(Box::new(bingo_sdk::Reason::Mode {
            mode: "acceptEdits".into(),
        }));
    }
    let mut state = with_permission_mode("acceptEdits");
    state.apply(&frame(
        2,
        bingo_sdk::Event::ItemCompleted { item: call.clone() },
    ));
    state.apply(&frame(
        3,
        bingo_sdk::Event::ItemCompleted {
            item: item(
                "receipt",
                ItemStatus::Completed,
                ItemBody::PermissionReceipt {
                    item: Some(call.id.clone()),
                    interaction: bingo_sdk::InteractionId::from_raw("ask"),
                    tool: "Inspect".into(),
                    decision: DecisionKind::AllowChanged,
                    feedback: None,
                    asked_at: Some(ts()),
                },
            ),
        },
    ));
    let value = step(&call, &state, &Default::default());
    assert_eq!(value.answer, Some(Upshot::plain("changed by you")));
    assert_eq!(value.why.as_deref(), Some("accept edits"));
}
