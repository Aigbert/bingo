//! Client actions returned in command receipts.
use super::*;
pub(super) fn action(run: &mut Run, action: bingo_sdk::Action) {
    let now = run.detection.now();
    match action.name.as_str() {
        "Browse" => crate::browse::open(&mut run.ui, &run.session.tree, None, false, now),
        "Composer" => {
            if let Ok(fill) = serde_json::from_value(action.args) {
                crate::composer::fill::fill(&mut run.ui, fill, now);
            }
        }
        "Pager" => page(run, &action.args, now),
        "Pane" => focus(run, &action.args),
        "Plugins" => plugins(run, &action.args, now),
        _ => palette::surface_action(run, action),
    }
}

fn page(run: &mut Run, args: &Value, now: crate::clock::Now) {
    if let Ok(page) = serde_json::from_value(args.clone()) {
        crate::pager::page::open(&mut run.ui, &run.session.tree, page, now);
    }
}
/// `/plugins` answers with the panel to open and the group it opens on.
fn plugins(run: &mut Run, args: &Value, now: crate::clock::Now) {
    if let Ok(first) = serde_json::from_value(args["open"].clone()) {
        crate::plugins::open(&mut run.ui, first, now);
    }
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
        let now = run.detection.now();
        crate::composer::fill::fill(&mut run.ui, crate::composer::fill::Fill { text }, now);
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
