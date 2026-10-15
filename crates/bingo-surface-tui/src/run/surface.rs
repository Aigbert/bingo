use super::*;
pub(super) fn action(run: &mut Run, action: bingo_sdk::Action) {
    let now = run.detection.now();
    match action.name.as_str() {
        "Browse" => crate::browse::open(&mut run.ui, &run.session.tree, None, false, now),
        "Composer" => {
            if let Some(text) = action.args["text"].as_str() {
                crate::browse::composer(&mut run.ui, text, now);
            }
        }
        "Pager" => {
            if let Some(id) = action.args["item"].as_str() {
                let id = bingo_sdk::ItemId::from_raw(id);
                if run.session.tree.viewed().transcript.get(&id).is_some() {
                    run.ui
                        .focus
                        .show(Open::Pager(crate::pager::Pager::open(id)), now.instant);
                }
            }
        }
        _ => palette::surface_action(run, action),
    }
}

pub(super) fn restore_prompt(run: &mut Run, text: Option<String>) {
    if let Some(text) = text {
        action(
            run,
            bingo_sdk::Action {
                name: "Composer".into(),
                args: serde_json::json!({"text":text}),
            },
        );
    }
}

pub(super) fn undone(run: &Run, frame: &bingo_sdk::Frame) -> Option<String> {
    let Event::Rewound { to_turn, .. } = &frame.event else {
        return None;
    };
    if &frame.session != run.session.tree.view() {
        return None;
    }
    let own = run.mine.values().flatten().any(|line| {
        let Some((name, args)) = crate::commands::split(line) else {
            return false;
        };
        name == "rewind" && args.split_whitespace().next() == Some(to_turn.as_str())
    });
    if !own {
        return None;
    }
    bingo_sdk::transcript::turns(&run.session.tree.viewed().transcript)
        .into_iter()
        .find(|t| &t.id == to_turn)
        .and_then(|t| t.asked)
}

pub(super) fn is_rewind(line: &str) -> bool {
    crate::commands::split(line).is_some_and(|(name, _)| name == "rewind")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        doubles::{Recorder, TestSession},
        test_support::*,
    };
    use std::sync::Arc;
    #[tokio::test]
    async fn only_own_rewind_returns_the_full_prompt_and_keeps_a_draft() {
        for (own, draft, queued) in [
            (true, "", false),
            (true, "draft", false),
            (false, "", false),
            (true, "", true),
        ] {
            let now = still(scene().1);
            let mut state = state();
            state.transcript = vec![user("i1", "first\nsecond")].into_iter().collect();
            let session = Arc::new(TestSession::default());
            let mut run = super::super::tests::idle_in(state, session, now.instant);
            run.ui.composer.set(draft);
            if own {
                let intent = run.mint(Some("/rewind trn_1 conversation".into()));
                if queued {
                    run.ack(&intent, &IntentOutcome::Queued { position: 1 });
                }
            }
            let event = frame(
                1,
                Event::Rewound {
                    generation: 1,
                    to_turn: bingo_sdk::TurnId::from_raw("trn_1"),
                    dropped: vec![bingo_sdk::ItemId::from_raw("i1")],
                    files_restored: vec![],
                },
            );
            let mut screen = Recorder::default();
            run.frame(&event, &mut screen).unwrap();
            assert_eq!(
                run.ui.composer.text(),
                if own && draft.is_empty() {
                    "first\nsecond"
                } else {
                    draft
                }
            );
        }
    }
}
