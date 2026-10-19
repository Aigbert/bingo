use crate::opening::Opening;
use bingo_sdk::*;
use serde_json::{Value, json};

fn context() -> (testing::TestHost, HookContext) {
    let host = testing::TestHost::default();
    host.on_signal(|_, _, _, _| async { Ok(()) });
    let cx = HookContext {
        session: SessionId::from_raw("ses_parent"),
        turn: None,
        item: None,
        cwd: "/unused".into(),
        provider: None,
        model: None,
        host: host.handle(),
    };
    (host, cx)
}

fn fallback() -> CommandOutcome {
    CommandOutcome::View {
        view: View::Table {
            headers: vec!["name".into()],
            rows: vec![vec!["plain result".into()]],
        },
    }
}

async fn opened(input: Value, plain: CommandOutcome) -> (Value, crate::Shared, testing::TestHost) {
    let (host, cx) = context();
    let shared = crate::Shared::default();
    let answer = Opening(shared.clone())
        .chain(
            catalogue::COMMAND_RUN,
            &cx,
            input,
            Next::new(move |_| {
                let plain = plain.clone();
                Box::pin(async move { Ok(json!(plain)) })
            }),
        )
        .await
        .unwrap();
    (answer, shared, host)
}

#[tokio::test]
async fn bare_agents_opens_the_sheet_and_keeps_the_table_as_text() {
    for surface in ["terminal", "desktop"] {
        let (answer, shared, _) = opened(
            json!({"command":"agents","args":"","surface":surface}),
            fallback(),
        )
        .await;
        let CommandOutcome::Surface {
            action,
            fallback: text,
        } = serde_json::from_value(answer).unwrap()
        else {
            panic!("bare /agents opens the crew sheet");
        };
        assert_eq!(action.name, "Pane");
        assert_eq!(action.args, json!({"plugin":crate::ID,"id":"crew"}));
        let CommandOutcome::View { view } = fallback() else {
            unreachable!()
        };
        assert_eq!(text, Some(view.fold_markdown()));
        assert!(
            shared
                .lock()
                .unwrap()
                .contains_key(&SessionId::from_raw("ses_parent"))
        );
    }
}

#[tokio::test]
async fn an_empty_roster_still_opens_and_preserves_the_plain_message() {
    let message = CommandOutcome::Applied {
        message: Some("no agents are running in this session".into()),
    };
    let (answer, _, _) = opened(
        json!({"command":"agents","args":"","surface":"terminal"}),
        message,
    )
    .await;
    let CommandOutcome::Surface { fallback, .. } = serde_json::from_value(answer).unwrap() else {
        panic!("an empty roster still opens the sheet");
    };
    assert_eq!(
        fallback.as_deref(),
        Some("no agents are running in this session")
    );
}

#[tokio::test]
async fn arguments_and_nonvisual_callers_keep_the_plain_command() {
    for input in [
        json!({"command":"agents","args":"stop x","surface":"terminal"}),
        json!({"command":"agents","args":""}),
        json!({"command":"agents","args":"","surface":"text"}),
    ] {
        let (answer, shared, host) = opened(input, fallback()).await;
        assert_eq!(answer, json!(fallback()));
        assert!(shared.lock().unwrap().is_empty());
        assert!(host.calls().is_empty());
    }
}

#[tokio::test]
async fn crew_opens_the_sheet_and_takes_no_words() {
    let (_, hook) = context();
    let cx = CommandContext {
        surface: None,
        session: hook.session,
        cwd: hook.cwd,
        host: hook.host,
        cancel: Default::default(),
    };
    let shared = crate::Shared::default();
    let command = crate::command::Open(shared.clone());
    let CommandOutcome::Surface { action, .. } = command.run("", &cx).await.unwrap() else {
        panic!("/crew opens the sheet");
    };
    assert_eq!(action.name, "Pane");
    assert!(shared.lock().unwrap().contains_key(&cx.session));
    assert_eq!(command.spec().args, ArgSpec::None);
    assert!(command.spec().keys.is_empty());
}

#[test]
fn a_sheet_redraws_for_the_sessions_it_watches() {
    use std::collections::BTreeSet;
    let watching = BTreeSet::from([SessionId::from_raw("ses_scout")]);
    let started = |session: &str| -> Frame {
        serde_json::from_value(json!({
            "seq": 1,
            "ts": "2026-10-19T00:00:00Z",
            "session": session,
            "event": {"type":"turnStarted","turn":"trn_1","inputs":[],"origin":"submit"},
        }))
        .unwrap()
    };
    assert!(crate::refresh::relevant(&watching, &started("ses_scout")));
    assert!(!crate::refresh::relevant(&watching, &started("ses_other")));
}
