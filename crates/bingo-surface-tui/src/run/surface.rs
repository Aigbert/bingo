//! Client actions returned in command receipts.
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
        "Pager" => page(run, &action.args, now),
        "Pane" => focus(run, &action.args),
        _ => palette::surface_action(run, action),
    }
}

fn page(run: &mut Run, args: &Value, now: crate::clock::Now) {
    if let Some(id) = args["item"].as_str() {
        let id = bingo_sdk::ItemId::from_raw(id);
        if run.session.tree.viewed().transcript.get(&id).is_some() {
            crate::pager::show(&mut run.ui, id, now);
        }
        return;
    }
    let (Some(title), Some(text)) = (args["title"].as_str(), args["text"].as_str()) else {
        return;
    };
    let return_to = match &run.ui.focus {
        crate::focus::Focus::Pane(key) => Some(crate::pager::Return::Pane(key.clone())),
        _ => None,
    };
    let pager = crate::pager::Pager::text(title.into(), text.into(), return_to);
    run.ui.focus.show(Open::Pager(pager), now.instant);
}
fn focus(run: &mut Run, args: &Value) {
    let (Some(plugin), Some(id)) = (args["plugin"].as_str(), args["id"].as_str()) else {
        return;
    };
    if let Some(pane) = crate::pane::placed(run.session.tree.viewed())
        .into_iter()
        .find(|pane| pane.plugin == plugin && pane.id == id && pane.asked)
    {
        run.ui.focus = crate::focus::Focus::Pane(pane.key);
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

#[derive(Default)]
pub(super) struct Prompts(
    std::collections::BTreeMap<(SessionId, bingo_sdk::TurnId), (IntentId, String)>,
);
impl Prompts {
    pub fn cancel(&mut self, intent: &IntentId) {
        self.0.retain(|_, (pending, _)| pending != intent);
    }
}
pub(super) fn capture(run: &mut Run, intent: &IntentId, line: Option<&str>) {
    let Some(("rewind", args)) = line.and_then(crate::commands::split) else {
        return;
    };
    let mut words = args.split_whitespace();
    let Some(target) = words.next() else {
        return;
    };
    if words.next() == Some("files") {
        return;
    }
    let transcript = &run.session.tree.viewed().transcript;
    if let Some(turn) = bingo_sdk::transcript::turns(transcript)
        .into_iter()
        .find(|turn| turn.id.as_str() == target)
        && let Some(text) = turn.asked(transcript).map(str::to_owned)
    {
        run.rewind_prompts.0.insert(
            (run.session.tree.view().clone(), turn.id),
            (intent.clone(), text),
        );
    }
}
pub(super) fn undone(run: &mut Run, frame: &bingo_sdk::Frame) -> Option<String> {
    let Event::Rewound { to_turn, .. } = &frame.event else {
        return None;
    };
    let (_, text) = run
        .rewind_prompts
        .0
        .remove(&(frame.session.clone(), to_turn.clone()))?;
    (&frame.session == run.session.tree.view()).then_some(text)
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
                run.ack(
                    &intent,
                    &IntentOutcome::Applied {
                        result: serde_json::json!({}),
                    },
                );
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
