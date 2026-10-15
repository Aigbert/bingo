use bingo_sdk::*;
use serde_json::json;

#[tokio::test]
async fn rendering_an_open_pane_retains_its_subscription() {
    let host = testing::TestHost::default();
    host.on_catalog(|kind| async move {
        Ok(Catalog {
            kind,
            entries: vec![],
        })
    });
    let shared = crate::Shared::default();
    let session = SessionId::from_raw("ses_viewed");
    shared
        .lock()
        .unwrap()
        .insert(session.clone(), Default::default());
    let cx = HookContext {
        session: session.clone(),
        turn: None,
        item: None,
        cwd: "/unused".into(),
        provider: None,
        model: None,
        host: host.handle(),
    };
    let answer = crate::draw::Drawing(shared.clone())
        .chain(
            catalogue::UI_RENDER,
            &cx,
            json!({"component":"Pane","requestId":"crew","surface":"terminal",
            "props":{},"viewport":{"columns":80,"rows":18}}),
            Next::new(|_| Box::pin(async { Ok(json!(View::text("fallback"))) })),
        )
        .await
        .unwrap();
    let view: View = serde_json::from_value(answer).unwrap();
    assert!(view.fold().contains("No agents here yet."));
    assert!(shared.lock().unwrap().contains_key(&session));
}

fn frame(event: Event) -> Frame {
    Frame {
        seq: Seq(1),
        ts: "2026-10-15T00:00:00Z".parse().unwrap(),
        session: SessionId::from_raw("ses_child"),
        cause: None,
        event,
    }
}
#[test]
fn only_display_changes_invalidate_a_followed_card() {
    let mut state = SessionState::new(crate::cards_tests::row("reviewer", "idle").summary);
    for event in [
        Event::Invalidated {
            requests: vec!["other".into()],
        },
        Event::Notice {
            level: Level::Info,
            code: "note".into(),
            text: "unrelated".into(),
        },
    ] {
        assert!(!crate::live::apply(&mut state, &frame(event)));
    }
    let mut summary = state.summary.clone();
    summary.model = Some("new-model".into());
    assert!(crate::live::apply(
        &mut state,
        &Frame {
            seq: Seq(2),
            ..frame(Event::SessionUpdated { summary })
        }
    ));
}
#[tokio::test]
async fn closed_panes_receive_no_refreshes_and_unrelated_events_are_ignored() {
    let host = testing::TestHost::default();
    let session = SessionId::from_raw("ses_parent");
    let cx = HookContext {
        session: session.clone(),
        turn: None,
        item: None,
        cwd: "/unused".into(),
        provider: None,
        model: None,
        host: host.handle(),
    };
    let shared = crate::Shared::default();
    let refresh = crate::refresh::Refresh(shared.clone());
    let event = frame(Event::Extension {
        plugin: "bingo.tasks".into(),
        kind: "tasks".into(),
        payload: serde_json::json!({}),
    });
    refresh.on_event(&event, &cx).await;
    assert!(host.calls().is_empty());
    shared.lock().unwrap().insert(
        session.clone(),
        crate::state::Panel {
            watching: [event.session.clone()].into(),
            ..Default::default()
        },
    );
    refresh
        .on_event(
            &frame(Event::Invalidated {
                requests: vec!["crew".into()],
            }),
            &cx,
        )
        .await;
    assert!(host.calls().is_empty());
    refresh.on_event(&event, &cx).await;
    assert_eq!(host.calls().len(), 1);
    refresh
        .on_event(
            &Frame {
                session: session.clone(),
                ..frame(Event::SessionClosed {
                    reason: CloseReason::Shutdown,
                })
            },
            &cx,
        )
        .await;
    assert!(!shared.lock().unwrap().contains_key(&session));
    refresh.on_event(&event, &cx).await;
    assert_eq!(host.calls().len(), 1);
}

#[tokio::test]
async fn an_inflight_render_cannot_resurrect_a_cancelled_confirmation() {
    let (host, entered, release) = paused_catalogue();
    let shared = crate::Shared::default();
    let session = SessionId::from_raw("ses_parent");
    shared.lock().unwrap().insert(
        session.clone(),
        crate::state::Panel {
            confirm: Some(crate::state::Confirmation::Dismiss("reviewer".into())),
            ..Default::default()
        },
    );
    let cx = HookContext {
        session: session.clone(),
        turn: None,
        item: None,
        cwd: "/unused".into(),
        provider: None,
        model: None,
        host: host.handle(),
    };
    let drawing = crate::draw::Drawing(shared.clone());
    let render = drawing.chain(
        catalogue::UI_RENDER,
        &cx,
        json!({"component":"Pane","requestId":"crew","surface":"terminal",
            "props":{},"viewport":{"columns":80,"rows":18}}),
        Next::new(|_| Box::pin(async { Ok(json!(View::text("fallback"))) })),
    );
    let key = async {
        entered.notified().await;
        crate::actions::Handler(shared.clone())
            .run(
                &json!({"handler":"crew","event":"press","value":{"key":"esc"}}).to_string(),
                &CommandContext {
                    session: session.clone(),
                    cwd: "/unused".into(),
                    host: host.handle(),
                    cancel: Default::default(),
                },
            )
            .await
            .unwrap();
        release.notify_one();
    };
    let (view, ()) = tokio::join!(render, key);
    assert!(shared.lock().unwrap()[&session].confirm.is_none());
    let view: View = serde_json::from_value(view.unwrap()).unwrap();
    assert!(!view.fold().contains("delete its journal"));
    assert_eq!(shared.lock().unwrap()[&session].watching.len(), 1);
}

fn paused_catalogue() -> (
    testing::TestHost,
    std::sync::Arc<tokio::sync::Notify>,
    std::sync::Arc<tokio::sync::Notify>,
) {
    let host = testing::TestHost::default();
    let entered = std::sync::Arc::new(tokio::sync::Notify::new());
    let release = std::sync::Arc::new(tokio::sync::Notify::new());
    let (began, resume) = (entered.clone(), release.clone());
    host.on_catalog(move |kind| {
        let (began, resume) = (began.clone(), resume.clone());
        async move {
            began.notify_one();
            resume.notified().await;
            Ok(Catalog {
                kind,
                entries: vec![],
            })
        }
    });
    (host, entered, release)
}

#[test]
fn input_commits_leave_render_subscriptions_in_place() {
    let mut current = crate::state::Panel::default();
    let mut input = current.clone();
    current.watching.insert(SessionId::from_raw("ses_new"));
    input.selected = 1;
    input.confirm = Some(crate::state::Confirmation::CloseRoom("design".into()));
    current.apply_input(&input);
    assert_eq!(current.selected, 1);
    assert_eq!(current.confirm, input.confirm);
    assert!(current.watching.contains(&SessionId::from_raw("ses_new")));
}
