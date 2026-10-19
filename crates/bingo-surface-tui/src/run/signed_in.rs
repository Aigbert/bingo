//! A sign-in that ends well sends again what the expired one refused.
use super::*;

/// The message that is kept is the refused turn's prompt; nothing holds a copy.
pub(super) fn after(run: &mut Run, submission: &Submission, result: &Value) {
    let Submission::Ordinary(Some(line)) = submission else {
        return;
    };
    if crate::commands::split(line).map(|(name, _)| name)
        != Some(crate::component::signed_out::COMMAND)
    {
        return;
    }
    let state = run.session.tree.viewed();
    let Some(provider) = signed_in_to(state, result) else {
        return;
    };
    let kept = (state.summary.provider.as_deref() == Some(provider.as_str()))
        .then(|| refused(state))
        .flatten();
    let now = run.now().instant;
    let Some(text) = kept else {
        let said = bingo_sdk::toast::Say::ok(format!("signed in to {provider}"));
        return run.ui.tell(said, now);
    };
    run.ui.tell(
        bingo_sdk::toast::Say::ok("signed in · sending your message again"),
        now,
    );
    run.submit(Input::text(text, crate::origin()));
}

/// The provider the recorded receipt names; no receipt, no sign-in.
fn signed_in_to(state: &SessionState, result: &Value) -> Option<String> {
    let id = result.get("item")?.as_str()?;
    let item = state
        .transcript
        .items()
        .iter()
        .find(|item| item.id.as_str() == id)?;
    match &item.body {
        bingo_sdk::ItemBody::Action { name, args, .. }
            if name == crate::component::signed_out::COMMAND =>
        {
            args.as_str().map(str::to_owned)
        }
        _ => None,
    }
}

fn refused(state: &SessionState) -> Option<String> {
    crate::component::signed_out::props(state)?;
    let turns = bingo_sdk::transcript::turns(&state.transcript);
    turns.last()?.asked(&state.transcript).map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doubles::TestSession;
    use crate::test_support::*;
    use bingo_sdk::{ErrorCode, ItemBody, ItemStatus, KernelError, TurnId, TurnStatus};
    use serde_json::json;

    fn signed_out(provider: &str) -> SessionState {
        let mut prompt = user("prompt", "fix it and add a test");
        prompt.turn = Some(TurnId::from_raw("t1"));
        let receipt = loose(item(
            "receipt",
            ItemStatus::Completed,
            ItemBody::Action {
                name: "login".into(),
                args: json!(provider),
                result: Some(json!("Signed in.")),
            },
        ));
        let failed = TurnStatus::Failed {
            error: KernelError::new(ErrorCode::AuthRequired, "the token expired"),
        };
        let mut state = folded(vec![
            frame(1, started("t1")),
            frame(2, Event::ItemCompleted { item: prompt }),
            frame(3, completed("t1", failed)),
            frame(4, Event::ItemCompleted { item: receipt }),
        ]);
        state.summary.provider = Some("anthropic".into());
        state
    }

    fn signed_in(provider: &str) -> (Run, std::sync::Arc<TestSession>, tempfile::TempDir) {
        let session = std::sync::Arc::new(TestSession::default());
        let mut run = super::super::tests::idle_in(
            signed_out(provider),
            session.clone(),
            std::time::Instant::now(),
        );
        // A sent line joins the prompt history, which lives in the data dir.
        let data = tempfile::tempdir().unwrap();
        run.data_dir = data.path().to_owned();
        let submission = Submission::Ordinary(Some("/login".into()));
        after(&mut run, &submission, &json!({"item": "receipt"}));
        (run, session, data)
    }

    fn told(run: &Run) -> String {
        let Some(crate::ui::NoticeKind::Message(message)) =
            run.ui.notice.as_ref().map(|notice| &notice.kind)
        else {
            panic!("no toast")
        };
        message.parts.iter().map(|part| part.fold()).collect()
    }

    #[test]
    fn signing_in_again_sends_the_refused_prompt_once() {
        let (run, session, _data) = signed_in("anthropic");
        let sent = session.submitted();
        assert!(
            matches!(sent.as_slice(), [Input::Text { text, .. }] if text == "fix it and add a test"),
            "{sent:?}"
        );
        assert_eq!(told(&run), "signed in · sending your message again");
    }

    #[test]
    fn signing_in_elsewhere_only_says_so() {
        let (run, session, _data) = signed_in("openai");
        assert!(session.submitted().is_empty());
        assert_eq!(told(&run), "signed in to openai");
    }
}
