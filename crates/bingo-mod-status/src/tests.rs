use super::*;
use serde_json::json;

fn state() -> SessionState {
    let Event::SessionUpdated { summary } = serde_json::from_str(include_str!(
        "../../bingo-sdk/fixtures/record/event/sessionUpdated.json"
    ))
    .unwrap() else {
        panic!("summary")
    };
    SessionState::new(summary)
}
#[test]
fn rows_match_the_prototype_scenarios() {
    let mut state = state();
    state.config.plugins.insert(
        "bingo.permissions".into(),
        json!({"mode":"acceptEdits","rules":["Edit(src/**)","Bash(cargo:*)"]}),
    );
    for (kind, word, tone) in [
        ("on", "1 on", rows::Tone::Ok),
        ("failed", "failed", rows::Tone::Bad),
        ("waiting", "waits for you", rows::Tone::Warn),
        ("pending", "pending", rows::Tone::Warn),
    ] {
        let plugins = vec![CatalogEntry {
            id: "fixture".into(),
            label: "fixture".into(),
            meta: json!({"state":{"kind":kind}}),
        }];
        let rows = rows::rows(
            &(&state).into(),
            &plugins,
            &[serde_json::from_value(json!({
                "name": "remote",
                "transport": {"type": "http", "url": "https://mcp.example.com"},
                "status": {"kind": "needsAuth", "why": "signed out"}
            }))
            .unwrap()],
            None,
        );
        let plugin = rows.iter().find(|r| r.label == "plugins").unwrap();
        assert!(
            plugin
                .value
                .iter()
                .any(|s| s.text.contains(word) && s.tone == tone)
        );
        assert!(
            rows.last()
                .unwrap()
                .value
                .iter()
                .any(|s| s.text.contains("remote needs sign-in"))
        );
        assert_eq!(
            rows.iter()
                .filter_map(|r| r.command.as_deref())
                .collect::<Vec<_>>(),
            [
                "rename",
                "model",
                "compact",
                "permission",
                "permission forget Edit(src/**)",
                "permission forget Bash(cargo:*)",
                "plugins",
                "mcp"
            ]
        );
    }
}
#[test]
fn drawing_is_a_valid_status_answer_with_an_owned_root_handler() {
    let rows = rows::rows(&(&state()).into(), &[], &[], None);
    for (width, height) in [(108, 15), (80, 12)] {
        let tree = draw::tree(&rows, "context", width, height, 2520);
        assert!(view::answer_ok(render::Component::Status, &json!({}), ID, None, &tree).is_ok());
        assert!(tree.fold().contains("live; ⏎ on a row goes there"));
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
    live::Refresh(selections.clone())
        .on_event(&frame, &cx)
        .await;
    assert_eq!(selections.lock().unwrap()[&state.summary.id], "plugins");
}
#[test]
fn a_server_row_reads_the_state_the_plugins_panel_reads() {
    let servers: Vec<bingo_services::mcp::ServerRow> = serde_json::from_value(json!([
        {"name": "github", "transport": {"type": "http", "url": "https://mcp.example.com"},
         "status": {"kind": "connected", "tools": 8}},
        {"name": "local", "transport": {"type": "stdio", "command": "npx"},
         "origin": "this project · /work", "status": {"kind": "waiting", "changed": false}}
    ]))
    .unwrap();
    let rows = rows::rows(&(&state()).into(), &[], &servers, None);
    let mcp = rows.iter().find(|r| r.label == "mcp").unwrap();
    assert_eq!(mcp.value[0].text, "1 connected · 8 tools");
    let waiting = servers[1].status.said();
    assert_eq!(waiting, "waiting for your yes");
    assert_eq!(mcp.value[1].text, format!(" · local {waiting}"));
    assert_eq!(mcp.value[1].tone, rows::Tone::Warn);
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
        started: Instant::now(),
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
            .contains("session")
    );
    assert!(
        !host
            .calls()
            .iter()
            .any(|call| matches!(call, testing::HostCall::Open { .. }))
    );
}
