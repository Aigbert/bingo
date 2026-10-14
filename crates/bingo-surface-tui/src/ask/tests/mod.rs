use super::*;
use crate::test_support::*;
use bingo_sdk::{
    Answer, AnswerSpec, Event, InteractionKind, ItemStatus, Preview, component::Props,
};
use serde_json::json;

fn scene_for(mut open: Interaction, command: bool) -> (crate::tree::Tree, Ui, crate::clock::Now) {
    if matches!(open.kind, InteractionKind::Permission { .. }) {
        open.answers.push(AnswerSpec::AllowChanged);
    }
    let input = if command {
        json!({"command": match &open.kind { InteractionKind::Permission { preview: Some(Preview::Command { command,.. }), .. } => command.as_str(), _ => "cargo test" }})
    } else {
        json!({"file_path":"src/main.rs","old_string":"old","new_string":"new"})
    };
    if let InteractionKind::Permission { input: current, .. } = &mut open.kind
        && current.is_null() { *current = input.clone(); }
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
    ui.dialog.focus_on(state.interactions.first());
    (solo(&state), ui, still(now))
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
        &key.request.request_id,
        &key.session,
    );
    ui.placed.pulled.request(&key);
    ui.placed.pulled.answer(key, view);
}
#[test]
fn prototype_scenarios_at_both_sizes() {
    for (width, height) in [(108, 30), (80, 24)] {
        for scenario in [
            "edit", "bash", "danger", "question", "multi", "form", "aside", "crew",
        ] {
            let mut open = match scenario {
                "question" | "multi" | "crew" => question(scenario == "multi", true),
                "form" => interaction(InteractionKind::Form { title: Some("repository".into()), questions: vec![serde_json::from_value(json!({"question":"Title","options":[],"freeText":true,"multi":false})).unwrap(), serde_json::from_value(json!({"question":"Draft","options":[{"id":"yes","label":"yes"}],"multi":true})).unwrap()] }, vec![AnswerSpec::Form, AnswerSpec::Cancel]),
                "bash" | "danger" => permission(Some("Bash(cargo:*)"), Some(Preview::Command { command: if scenario == "danger" { "rm -rf build" } else { "cargo test" }.into(), cwd:"/work".into(),description:None })),
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
                    *caution = Some("recursive deletion can permanently remove files".into());
                    *session_scope = None;
                }
            }
            let (mut tree, mut ui, now) = scene_for(open, matches!(scenario, "bash" | "danger"));
            if scenario == "aside" {
                crate::input::on_key(&mut ui, &tree, typed('x'), now);
            }
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
fn change_prefills_then_submits_and_plain_typing_sets_aside() {
    let mut open = permission(None, None);
    if let InteractionKind::Permission { tool, .. } = &mut open.kind {
        *tool = "Bash".into();
    }
    let (tree, mut ui, now) = scene_for(open, true);
    draw_tree(80, 24, &tree, &mut ui, now);
    assert!(crate::input::on_key(&mut ui, &tree, typed('3'), now).is_empty());
    assert_eq!(ui.dialog.words.as_ref().unwrap().text(), "cargo test");
    ui.dialog.words.as_mut().unwrap().set("cargo check");
    let effects = crate::input::on_key(&mut ui, &tree, key(KeyCode::Enter), now);
    assert!(
        matches!(&effects[0],crate::effect::Effect::Answer {answer:Answer::AllowChanged {input},..} if input["command"]=="cargo check")
    );
    let (tree, mut ui, now) = scene_for(permission(None, None), false);
    assert!(crate::input::on_key(&mut ui, &tree, typed('x'), now).is_empty());
    assert!(ui.dialog.aside);
    assert_eq!(ui.composer.text(), "x");
    assert!(crate::input::on_key(&mut ui, &tree, ctrl('g'), now).is_empty());
    assert!(!ui.dialog.aside);
}

#[test]
fn caution_starts_on_no_and_escape_denies_and_stops() {
    let mut open = permission(None, None);
    open.guard_until = Some(ts() + jiff::SignedDuration::from_secs(60));
    if let InteractionKind::Permission { caution, .. } = &mut open.kind {
        *caution = Some("destructive".into());
    }
    let (tree, mut ui, now) = scene_for(open, false);
    let screen = draw_tree(80, 24, &tree, &mut ui, now);
    assert!(screen.contains("❯ 2  No"));
    let effects = crate::input::on_key(&mut ui, &tree, key(KeyCode::Esc), now);
    assert!(matches!(
        effects.as_slice(),
        [crate::effect::Effect::StopInteraction {
            answer: Answer::Deny { .. },
            ..
        }]
    ));
}

#[test]
fn the_whole_preview_comes_from_the_child_that_asked() {
    let mut open = permission(
        None,
        Some(Preview::Diff {
            unified: (0..40).map(|n| format!("+child line {n}\n")).collect(),
        }),
    );
    open.session = child_id();
    let tree = folded_tree(vec![
        child_frame(1, announced("moss")),
        child_frame(
            2,
            started_tool(
                2,
                tool(
                    "itm_2",
                    "Edit",
                    json!({"new_string":"changed"}),
                    None,
                    ItemStatus::Pending,
                ),
            )
            .event,
        ),
        child_frame(3, opened(open)),
    ]);
    let (mut ui, now) = scene();
    let now = still(now);
    ui.dialog
        .focus_on(tree.open_interaction().map(|(_, open)| open));
    draw_tree(80, 24, &tree, &mut ui, now);
    crate::input::on_key(&mut ui, &tree, ctrl('o'), now);
    draw_tree(80, 24, &tree, &mut ui, now);
    crate::input::on_key(&mut ui, &tree, key(KeyCode::End), now);
    let screen = draw_tree(80, 24, &tree, &mut ui, now);
    assert!(screen.contains("child line 39"), "{screen}");
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
    crate::input::on_key(&mut ui, &tree, typed('3'), now);
    assert_eq!(ui.dialog.words.as_ref().unwrap().text(), "echo rewritten");
}
