use super::*;
use crate::test_support::*;
use bingo_sdk::{
    Answer, AnswerSpec, Event, InteractionKind, ItemStatus, Preview, component::Props,
};
use crossterm::event::KeyCode;
use serde_json::json;

fn preview_command(kind: &InteractionKind) -> &str {
    if let InteractionKind::Permission { preview, .. } = kind
        && let Some(Preview::Command { command, .. }) = preview.as_deref()
    {
        command
    } else {
        "cargo test"
    }
}

fn scene_for(mut open: Interaction, command: bool) -> (crate::tree::Tree, Ui, crate::clock::Now) {
    if matches!(open.kind, InteractionKind::Permission { .. }) {
        open.answers.push(AnswerSpec::AllowChanged);
    }
    let input = if command {
        json!({"command": preview_command(&open.kind)})
    } else {
        json!({"file_path":"src/main.rs","old_string":"old","new_string":"new"})
    };
    if let InteractionKind::Permission { input: current, .. } = &mut open.kind
        && current.is_null()
    {
        *current = input.clone();
    }
    let mut state = folded(vec![
        frame(1, started("trn_1")),
        started_tool(
            2,
            tool(
                "itm_2",
                if command { "Bash" } else { "Edit" },
                input,
                None,
                ItemStatus::Pending,
            ),
        ),
        frame(3, opened(open)),
    ]);
    state.config.kernel["render"] = json!({"Question":[{}]});
    let (mut ui, now) = scene();
    catalogue(&mut ui);
    ui.dialog.focus_on(state.interactions.first());
    let now = still(now);
    let read = crate::clock::Now {
        wall: now.wall + super::guard::GUARD,
        ..now
    };
    (solo(&state), ui, read)
}

/// Walk to the last answer, `Change it first`, and open it.
fn change(tree: &crate::tree::Tree, ui: &mut Ui, now: crate::clock::Now) {
    for _ in 0..3 {
        crate::input::on_key(ui, tree, key(KeyCode::Down), now);
    }
    assert!(crate::input::on_key(ui, tree, key(KeyCode::Enter), now).is_empty());
}
fn install_question(
    tree: &crate::tree::Tree,
    ui: &mut Ui,
    now: crate::clock::Now,
    width: u16,
    height: u16,
) {
    draw_tree(width, height, tree, ui, now);
    let Some(key) = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component == bingo_sdk::render::Component::Question)
        .cloned()
    else {
        return;
    };
    let Props::Question(props) =
        Props::parse(key.request.component, key.request.props.clone()).unwrap()
    else {
        panic!("question")
    };
    let view = bingo_mod_ask::drawing::tree(
        &props,
        &Default::default(),
        &bingo_mod_ask::drawing::Asked {
            request: &key.request.request_id,
            session: &key.session,
            columns: key.request.viewport.columns,
            glyphs: key.request.viewport.glyphs,
        },
    );
    ui.placed.pulled.request(&key);
    ui.placed.pulled.answer(key, view);
}
#[test]
fn prototype_scenarios_at_both_sizes() {
    for (width, height) in [(108, 30), (80, 24)] {
        for scenario in [
            "edit", "bash", "danger", "question", "multi", "form", "crew",
        ] {
            let mut open = match scenario {
                "question" | "multi" | "crew" => question(scenario == "multi", true),
                "form" => interaction(InteractionKind::Form { title: Some("repository".into()), questions: vec![serde_json::from_value(json!({"question":"Title","options":[],"freeText":true,"multi":false})).unwrap(), serde_json::from_value(json!({"question":"Draft","options":[{"id":"yes","label":"yes"}],"multi":true})).unwrap()] }, vec![AnswerSpec::Form, AnswerSpec::Cancel]),
                "bash" | "danger" => permission(Some("Bash(cargo:*)"), Some(Preview::Command { command: if scenario == "danger" { "rm -rf build" } else { "cargo test" }.into(), cwd:"/work".into(),description:None,stakes:None })),
                _ => permission(Some("Edit(src/)"), Some(Preview::Diff { unified:"-old\n+new".into() })),
            };
            if let InteractionKind::Permission {
                tool,
                caution,
                session_scope,
                ..
            } = &mut open.kind
            {
                if matches!(scenario, "bash" | "danger") {
                    *tool = "Bash".into();
                }
                if scenario == "danger" {
                    *caution = Some(bingo_sdk::Caution {
                        reason: "recursive deletion can permanently remove files".into(),
                        act: None,
                    });
                    *session_scope = None;
                }
            }
            let (mut tree, mut ui, now) = scene_for(open, matches!(scenario, "bash" | "danger"));
            if scenario == "crew" {
                tree = asking_crew();
                ui.dialog
                    .focus_on(tree.open_interaction().map(|(_, open)| open));
            }
            install_question(&tree, &mut ui, now, width, height);
            insta::assert_snapshot!(
                format!("m169_{scenario}_{width}x{height}"),
                draw_tree(width, height, &tree, &mut ui, now)
            );
        }
    }
}
fn asking_crew() -> crate::tree::Tree {
    let mut tree = folded_tree(vec![]);
    let mut config = tree.viewed().config.clone();
    config.kernel["render"] = json!({"Question":[{}]});
    for (n, name, delay) in [(4, "moss", 0), (2, "fern", 2), (3, "coda", 1)] {
        tree.fold
            .apply(&agent_frame(n, 1, agent_announced(n, name)));
        tree.fold.apply(&agent_frame(
            n,
            2,
            Event::ConfigChanged {
                config: config.clone(),
            },
        ));
        let mut open = question(false, true);
        open.id = bingo_sdk::InteractionId::from_raw(format!("int_{n}"));
        open.session = agent_id(n);
        open.item = None;
        open.opened_at = ts() + jiff::SignedDuration::from_millis(delay);
        tree.fold.apply(&agent_frame(n, 3, opened(open)));
    }
    tree
}

#[test]
fn change_prefills_then_submits_and_typing_starts_the_no() {
    let mut open = permission(None, None);
    if let InteractionKind::Permission { tool, .. } = &mut open.kind {
        *tool = "Bash".into();
    }
    let (tree, mut ui, now) = scene_for(open, true);
    draw_tree(80, 24, &tree, &mut ui, now);
    change(&tree, &mut ui, now);
    assert_eq!(ui.dialog.words.as_ref().unwrap().text(), "cargo test");
    crate::input::on_key(&mut ui, &tree, ctrl('u'), now);
    assert_eq!(ui.dialog.words.as_ref().unwrap().text(), "");
    for c in "cargo check".chars() {
        crate::input::on_key(&mut ui, &tree, typed(c), now);
    }
    let effects = crate::input::on_key(&mut ui, &tree, key(KeyCode::Enter), now);
    assert!(
        matches!(&effects[0],crate::effect::Effect::Answer {answer:Answer::AllowChanged {input},..} if input["command"]=="cargo check")
    );
    let (tree, mut ui, now) = scene_for(permission(None, None), false);
    assert!(crate::input::on_key(&mut ui, &tree, typed('x'), now).is_empty());
    assert_eq!(ui.dialog.words.as_ref().unwrap().text(), "x");
    assert!(!ui.dialog.aside);
    assert!(
        ui.composer.text().is_empty(),
        "the words are the no's, not the composer's"
    );
}

#[test]
fn caution_starts_on_no_and_escape_denies_and_stops() {
    let mut open = permission(None, None);
    if let InteractionKind::Permission { caution, .. } = &mut open.kind {
        *caution = Some(bingo_sdk::Caution {
            reason: "destructive".into(),
            act: None,
        });
    }
    let (tree, mut ui, now) = scene_for(open, false);
    let screen = draw_tree(80, 24, &tree, &mut ui, now);
    assert!(screen.contains("▎ No"), "{screen}");
    assert!(!screen.contains("for this session"), "{screen}");
    let effects = crate::input::on_key(&mut ui, &tree, key(KeyCode::Esc), now);
    assert!(matches!(
        effects.as_slice(),
        [crate::effect::Effect::StopInteraction {
            answer: Some(Answer::Deny { .. }),
            ..
        }]
    ));
}

#[test]
fn change_and_sealed_props_use_the_gates_rewritten_input() {
    let open = permission(None, None);
    let mut wire = serde_json::to_value(open).unwrap();
    wire["kind"]["input"] = json!({"command":"echo rewritten"});
    let (tree, mut ui, now) = scene_for(serde_json::from_value(wire).unwrap(), true);
    let (owner, open) = tree.open_interaction().unwrap();
    let Some(Props::Permission(props)) = crate::component::sealed::props(owner, open) else {
        panic!("permission")
    };
    assert_eq!(props.input, json!({"command":"echo rewritten"}));
    change(&tree, &mut ui, now);
    assert_eq!(ui.dialog.words.as_ref().unwrap().text(), "echo rewritten");
}

/// esc on words goes back to the list with the pick kept; esc on the list
/// says no and stops the turn.
#[test]
fn esc_leaves_the_words_first_and_the_ask_second() {
    for editing in [false, true] {
        let (tree, mut ui, now) = scene_for(permission(None, None), true);
        if editing {
            change(&tree, &mut ui, now);
            let screen = draw_tree(108, 30, &tree, &mut ui, now);
            assert!(screen.contains("esc back"), "{screen}");
        } else {
            crate::input::on_key(&mut ui, &tree, typed('x'), now);
        }
        let picked = ui.dialog.focus;
        let left = crate::input::on_key(&mut ui, &tree, key(KeyCode::Esc), now);
        assert!(left.is_empty(), "{left:?}");
        assert!(ui.dialog.words.is_none());
        assert_eq!(ui.dialog.focus, picked);
        assert!(matches!(
            crate::input::on_key(&mut ui, &tree, key(KeyCode::Esc), now).as_slice(),
            [crate::effect::Effect::StopInteraction {
                answer: Some(Answer::Deny { .. }),
                ..
            }]
        ));
    }
}

#[test]
fn a_click_on_a_choice_presses_it_once_the_guard_lets_keys_through() {
    let (tree, mut ui, read) = scene_for(question(false, true), false);
    install_question(&tree, &mut ui, read, 108, 30);
    draw_tree(108, 30, &tree, &mut ui, read);
    let target = ui
        .shown
        .sheet_actions
        .iter()
        .find(|target| target.item.action.args["value"]["pick"] == "1")
        .expect("the second choice is a click target");
    let mouse = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: target.area.x + 4,
        row: target.area.y,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };
    let early = crate::clock::Now {
        wall: read.wall - super::guard::GUARD,
        ..read
    };
    let (_, open) = tree.open_interaction().unwrap();
    assert!(!super::guard::open(
        super::guard::armed(&ui, open, early),
        early.wall
    ));
    assert!(crate::pointer::on_mouse(&mut ui, &tree, mouse, early).is_empty());
    assert!(matches!(
        crate::pointer::on_mouse(&mut ui, &tree, mouse, read).as_slice(),
        [crate::effect::Effect::SiteKey { action, .. }] if action.args["value"]["pick"] == "1"
    ));
}

#[test]
fn escaping_questions_and_forms_stops_the_turn() {
    for open in [question(false, true), form()] {
        let (tree, mut ui, now) = scene_for(open, false);
        let (owner, open) = tree.open_interaction().unwrap();
        assert_eq!(
            crate::input::on_key(&mut ui, &tree, key(KeyCode::Esc), now),
            vec![crate::effect::Effect::StopInteraction {
                session: owner.summary.id.clone(),
                interaction: open.id.clone(),
                answer: Some(Answer::Cancel),
            }]
        );
        assert!(ui.stop_asked.is_some());
        assert!(ui.dialog.answered);
    }
}

#[test]
fn changed_session_receipts_name_the_edit_and_the_rule() {
    let body: bingo_sdk::ItemBody = serde_json::from_value(json!({"kind":"permissionReceipt", "interaction":"i", "tool":"sample", "decision":"allowChangedSession"})).unwrap();
    assert_eq!(
        super::receipts::label(&body).as_deref(),
        Some("you changed it first; yes for this session")
    );
}

#[test]
fn a_single_answered_step_uses_the_singular() {
    let receipt = item(
        "answer",
        ItemStatus::Completed,
        bingo_sdk::ItemBody::QuestionAnswer {
            asked_at: None,
            item: Some(bingo_sdk::ItemId::from_raw("call")),
            interaction: bingo_sdk::InteractionId::from_raw("q"),
            question: "Form".into(),
            answer: "Beta".into(),
        },
    );
    let state = folded(vec![
        frame(1, started("trn_1")),
        frame(
            2,
            bingo_sdk::Event::ItemCompleted {
                item: tool(
                    "call",
                    "AskUserQuestion",
                    json!({}),
                    Some(bingo_sdk::ToolOutput::text("The user answered: Beta")),
                    ItemStatus::Completed,
                ),
            },
        ),
        frame(3, bingo_sdk::Event::ItemCompleted { item: receipt }),
        frame(4, completed("trn_1", bingo_sdk::TurnStatus::Completed)),
    ]);
    let (mut ui, now) = scene();
    let screen = draw_sized(108, 30, &state, &mut ui, still(now));
    assert!(screen.contains("• askuserquestion"), "{screen}");
    assert!(!screen.contains("1 step"), "{screen}");
    assert!(!screen.contains("Form"), "{screen}");
}
