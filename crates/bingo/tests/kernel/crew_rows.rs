//! A step that addressed a teammate is drawn by the crew while the crew is
//! on, and by the ledger in the tool's own look while it is off.
use bingo_sdk::*;
use serde_json::json;

async fn started(
    home: &std::path::Path,
    crew: bool,
) -> (std::sync::Arc<bingo_core::Host>, Attachment) {
    let env = Env::rooted(home);
    bingo_core::plugins::switch(&env, "bingo.mod.crew", crew).unwrap();
    let path = bingo_core::settings::user_path(&env);
    let mut config = bingo_core::HostConfig::new(env);
    config.layers.push(bingo_core::settings::Layer::new(
        bingo_core::settings::LayerOrigin::User,
        path.display().to_string(),
        bingo_core::settings::read_document(&path).unwrap(),
    ));
    let host = bingo_core::Host::build(
        vec![
            Box::new(bingo_mod_ledger::Ledger),
            Box::new(bingo_mod_crew::Crew::default()),
        ],
        config,
    )
    .await
    .unwrap();
    let attachment = host
        .open(
            SessionSelector::Create {
                spec: SessionSpec {
                    cwd: home.into(),
                    driver: Driver::Log,
                    ..Default::default()
                },
            },
            ClientIdentity {
                name: "test".into(),
                surface: "terminal".into(),
            },
            OpenOptions::default(),
        )
        .await
        .unwrap();
    (host, attachment)
}

fn spawned_scout() -> render::RenderRequest {
    serde_json::from_value(json!({
        "component": "Ledger",
        "requestId": "itm_1",
        "surface": "terminal",
        "props": {
            "steps": [{
                "mark": "@", "verb": "spawn", "target": "find every caller of drain()",
                "reads": "text", "tier": "change", "state": "done",
                "teammate": {"name": "scout", "ink": "crew1"}
            }],
            "marks": {"asked": "◆"}
        },
        "viewport": {"columns": 80, "rows": 1}
    }))
    .unwrap()
}

#[tokio::test]
async fn the_crew_draws_a_teammates_row_and_the_ledger_draws_it_without_the_crew() {
    for (crew, row) in [
        (true, "   @ scout   find every caller of drain()"),
        (false, "   @ spawn   find every caller of drain()"),
    ] {
        let home = tempfile::tempdir().unwrap();
        let (host, attachment) = started(home.path(), crew).await;
        let view = host
            .render(&attachment.session, spawned_scout())
            .await
            .unwrap();
        let text = view.fold();
        assert!(text.starts_with(row), "crew {crew}: {text}");
        assert_eq!(text.ends_with("started"), crew, "{text}");
        host.shutdown().await;
    }
}
