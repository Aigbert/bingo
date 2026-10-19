use super::*;
use serde_json::json;

pub(crate) fn state() -> SessionState {
    let Event::SessionUpdated { summary } = serde_json::from_str(include_str!(
        "../../bingo-sdk/fixtures/record/event/sessionUpdated.json"
    ))
    .unwrap() else {
        panic!("summary")
    };
    SessionState::new(summary)
}
#[test]
fn status_uses_the_shared_builder_and_omits_missing_facts() {
    let rows = rows::rows(
        &(&state()).into(),
        &[],
        &[],
        None,
        &Default::default(),
        Default::default(),
    );
    assert!(!rows.iter().any(|r| r.label == "cost"));
    assert_eq!(
        rows.iter().map(|r| r.label.as_str()).collect::<Vec<_>>(),
        [
            "model",
            "mode",
            "context",
            "directory",
            "plugins",
            "agents",
            "rules"
        ]
    );
    let tree = draw::tree(&rows, "plugins", 100, 18, Default::default());
    assert!(tree.fold().contains("Status"));
    assert!(
        serde_json::to_string(&tree)
            .unwrap()
            .contains("\"ground\":\"pick\"")
    );
}
#[test]
fn drawing_is_a_valid_status_answer_with_an_owned_root_handler() {
    let rows = rows::rows(
        &(&state()).into(),
        &[],
        &[],
        None,
        &Default::default(),
        Default::default(),
    );
    for (width, height) in [(108, 15), (80, 12)] {
        let tree = draw::tree(&rows, "context", width, height, Default::default());
        assert!(view::answer_ok(render::Component::Status, &json!({}), ID, None, &tree).is_ok());
        bingo_sdk::testing::assert_separate_hints(&tree);
    }
}
#[tokio::test]
async fn config_frames_invalidate_an_open_card_without_changing_selection() {
    let selections = Selections::default();
    let state = state();
    selections
        .lock()
        .unwrap()
        .insert(state.summary.id.clone(), "plugins".into());
    let host = testing::TestHost::default();
    let cx = HookContext {
        session: state.summary.id.clone(),
        turn: None,
        item: None,
        cwd: "/unused".into(),
        provider: None,
        model: None,
        host: host.handle(),
    };
    let frame: Frame = serde_json::from_value(json!({"session":state.summary.id, "seq":1,"ts":"2026-10-15T00:00:00Z","event":{"type":"configChanged","config":state.config}})).unwrap();
    let status = Status {
        selections: selections.clone(),
        ..Default::default()
    };
    let mut registrar = Registrar::new(ID, json!({}), Env::rooted("/unused"), Default::default());
    status.register(&mut registrar).unwrap();
    for contribution in registrar.into_contributions() {
        if let Contribution::Hook(hook) = contribution {
            hook.on_event(&frame, &cx).await;
        }
    }
    assert!(host.calls().iter().any(|call| matches!(call,
        testing::HostCall::Invalidate { session, requests } if session == &state.summary.id && requests == &[REQUEST.to_string()]
    )));
    assert_eq!(
        selections.lock().unwrap()[&state.summary.id].label,
        "plugins"
    );
}
#[tokio::test]
async fn rendering_uses_supplied_facts_without_opening_a_session() {
    let state = state();
    let host = testing::TestHost::default();
    host.on_catalog(|kind| async move {
        Ok(Catalog {
            kind,
            entries: vec![],
        })
    });
    let cx = HookContext {
        session: state.summary.id.clone(),
        turn: None,
        item: None,
        cwd: "/unused".into(),
        provider: None,
        model: None,
        host: host.handle(),
    };
    let drawing = draw::Drawing {
        selections: Default::default(),
        home: Default::default(),
    };
    let input = json!({"component":"Status","requestId":"status","surface":"terminal",
        "props":{"summary":state.summary,"config":state.config,"context":state.context},
        "viewport":{"columns":80,"rows":12}});
    let answer = drawing
        .chain(
            catalogue::UI_RENDER,
            &cx,
            input,
            Next::new(|_| Box::pin(async { Ok(json!(View::text("fallback"))) })),
        )
        .await
        .unwrap();
    assert!(
        serde_json::from_value::<View>(answer)
            .unwrap()
            .fold()
            .contains("Status")
    );
    assert!(
        !host
            .calls()
            .iter()
            .any(|call| matches!(call, testing::HostCall::Open { .. }))
    );
}

#[test]
fn malformed_plugin_states_are_reported_as_unavailable() {
    let entry = CatalogEntry {
        id: "invalid".into(),
        label: "invalid".into(),
        meta: json!({"state":{"kind":"failed"}}),
    };
    let rows = rows::rows(
        &(&state()).into(),
        &[entry],
        &[],
        None,
        &Default::default(),
        Default::default(),
    );
    let plugins = rows.iter().find(|row| row.label == "plugins").unwrap();
    assert!(
        plugins
            .value
            .iter()
            .any(|span| span.text.contains("state unavailable"))
    );
}

#[tokio::test]
async fn an_unmodified_pull_slot_renders_from_a_bounded_attachment() {
    let state = state();
    let host = testing::TestHost::with_sessions(vec![state.summary.clone()]);
    host.on_catalog(|kind| async move {
        Ok(Catalog {
            kind,
            entries: vec![],
        })
    });
    let cx = HookContext {
        session: state.summary.id,
        turn: None,
        item: None,
        cwd: "/unused".into(),
        provider: None,
        model: None,
        host: host.handle(),
    };
    let drawing = draw::Drawing {
        selections: Default::default(),
        home: Default::default(),
    };
    let input = json!({"component":"Status","requestId":"status","surface":"desktop",
        "props":{},"viewport":{"columns":80,"rows":12}});
    let answer = drawing
        .chain(
            catalogue::UI_RENDER,
            &cx,
            input,
            Next::new(|_| Box::pin(async { Ok(json!(View::text("fallback"))) })),
        )
        .await
        .unwrap();
    assert!(
        serde_json::from_value::<View>(answer)
            .unwrap()
            .fold()
            .contains("Status")
    );
    assert!(host.calls().iter().any(
        |call| matches!(call, testing::HostCall::Open { options, .. }
        if options.max_snapshot_bytes == Some(64 * 1024))
    ));
}

#[tokio::test]
async fn opener_returns_a_pane_receipt_each_time() {
    let host = testing::TestHost::default();
    host.on_signal(|_, _, _, _| async { Ok(()) });
    let cx = HookContext {
        session: state().summary.id,
        turn: None,
        item: None,
        cwd: "/unused".into(),
        provider: None,
        model: None,
        host: host.handle(),
    };
    let opener = command::Open(Default::default());
    for _ in 0..2 {
        let result = opener
            .chain(
                catalogue::COMMAND_RUN,
                &cx,
                json!({"command":"status","args":"","surface":"terminal"}),
                Next::new(|_| Box::pin(async { panic!("terminal opener fell through") })),
            )
            .await
            .unwrap();
        assert_eq!(result["kind"], "surface");
        assert_eq!(
            result["action"],
            json!({"name":"Pane","args":{"plugin":ID,"id":REQUEST}})
        );
    }
}

#[test]
fn mode_and_rules_come_only_from_the_configured_policy() {
    let mut state = state();
    state.config.kernel[ConfigView::POLICY] = json!("local.policy");
    state.config.plugins.insert(
        "bingo.permissions".into(),
        json!({"mode":"wrong","rules":["wrong"]}),
    );
    state.config.plugins.insert(
        "local.policy".into(),
        json!({"mode":"local","rules":["Read(local/**)"]}),
    );
    let rows = rows::rows(
        &(&state).into(),
        &[],
        &[],
        None,
        &Default::default(),
        Default::default(),
    );
    assert_eq!(
        rows.iter().find(|r| r.label == "mode").unwrap().value[0].text,
        "local"
    );
    assert_eq!(
        rows.iter().find(|r| r.label == "rules").unwrap().value[0].text,
        "Read(local/**)"
    );
}

#[test]
fn a_broken_server_appears_in_the_plugins_row() {
    let servers: Vec<bingo_services::mcp::ServerRow> = serde_json::from_value(json!([{
        "name":"github", "transport":{"type":"http","url":"https://example.test"},
        "status":{"kind":"needsAuth","why":"token expired"}
    }]))
    .unwrap();
    let rows = rows::rows(
        &(&state()).into(),
        &[],
        &servers,
        None,
        &Default::default(),
        Default::default(),
    );
    let plugins = rows.iter().find(|r| r.label == "plugins").unwrap();
    assert_eq!(plugins.value[0].text, "1 on");
    assert_eq!(plugins.value[2].text, "github token expired");
    assert_eq!(plugins.value[2].tone, rows::Tone::Warn);
}

#[test]
fn status_spells_all_separators_with_the_ascii_glyph_set() {
    let rows = rows::rows(
        &(&state()).into(),
        &[],
        &[],
        None,
        &Default::default(),
        glyph::GlyphSet::Ascii,
    );
    let view = draw::tree(&rows, "model", 100, 18, glyph::GlyphSet::Ascii);
    assert!(view.fold().is_ascii(), "{}", view.fold());
}
