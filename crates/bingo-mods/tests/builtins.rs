#![allow(clippy::unwrap_used, clippy::expect_used)]
use bingo_core::{Host, HostConfig};
use bingo_sdk::*;
use futures::{FutureExt, StreamExt};
use serde_json::json;
use std::{path::Path, sync::Arc};

async fn start(home: &Path, source: Option<&str>, disabled: bool) -> (Arc<Host>, Attachment) {
    let mut plugins: Vec<Box<dyn Plugin>> = vec![
        Box::new(bingo_mod_hello::Hello),
        Box::new(bingo_mod_ask::Ask),
    ];
    if let Some(source) = source {
        let root = home.join("outside");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(
            root.join("plugin.json"),
            json!({"name":"outside","version":"1","module":"main.ts","uses":["ui"]}).to_string(),
        )
        .unwrap();
        std::fs::write(root.join("main.ts"), source).unwrap();
        plugins.push(Box::new(
            bingo_mods::ModsPlugin::default()
                .with_dirs(vec![root])
                .with_project(home.into()),
        ));
    }
    let config = HostConfig::new(Env::rooted(home)).with_layer(
        "test",
        json!({"enabledPlugins":{"hello":!disabled,"ask":!disabled}}),
    );
    let host = Host::build(plugins, config).await.unwrap();
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
fn request() -> render::RenderRequest {
    serde_json::from_value(json!({"component":"Toast","requestId":"hello","surface":"terminal","props":{},"viewport":{"columns":80,"rows":4}})).unwrap()
}
async fn command(attachment: &mut Attachment, input: Input) -> IntentOutcome {
    let intent = IntentId::mint();
    attachment.handle.submit(intent.clone(), input);
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(frame) = attachment.events.next().await {
            if let Event::IntentAck {
                intent: id,
                outcome,
            } = frame.event
                && id == intent
            {
                return outcome;
            }
        }
        panic!("ack stream ended")
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn outside_wraps_or_replaces_the_builtin_in_a_temp_plugin_directory() {
    for (source, expected) in [
        (
            "export function register(on) { on('ui.render', {component:'Toast'}, async ($, e, next) => { const tree = await next(e); return {...tree, children:[h('Text',{},'Wrapped'), ...tree.children]}; }); }",
            "Wrapped\nHello from a built-in mod",
        ),
        (
            "export function register(on) { on('ui.render', {component:'Toast'}, () => h('Text',{},'Replacement')); }",
            "Replacement",
        ),
    ] {
        let home = tempfile::tempdir().unwrap();
        let (host, attachment) = start(home.path(), Some(source), false).await;
        let tree = host.render(&attachment.session, request()).await.unwrap();
        assert_eq!(tree.fold(), expected);
        host.shutdown().await;
    }
}
#[tokio::test]
async fn disabling_removes_the_command_binding_and_drawing() {
    let home = tempfile::tempdir().unwrap();
    let (host, mut attachment) = start(home.path(), None, true).await;
    let catalog = host.catalog(CatalogKind::Commands).await.unwrap();
    assert!(!catalog.entries.iter().any(|entry| entry.id == "hello"));
    let specs: Vec<CommandSpec> = catalog
        .entries
        .iter()
        .filter_map(|entry| serde_json::from_value(entry.meta.clone()).ok())
        .collect();
    assert!(
        !keys::bindings(&specs, &Default::default(), &[])
            .commands
            .contains_key("ctrl+x")
    );
    assert_eq!(
        host.render(&attachment.session, request()).await.unwrap(),
        request().engine()
    );
    assert!(matches!(
        command(
            &mut attachment,
            Input::text("/hello", Origin::surface("terminal"))
        )
        .await,
        IntentOutcome::Rejected { .. }
    ));
    host.shutdown().await;
}
#[tokio::test]
async fn rust_render_is_ready_on_the_calling_thread_and_handler_results_round_trip() {
    let home = tempfile::tempdir().unwrap();
    let (host, mut attachment) = start(home.path(), None, false).await;
    let view = host
        .render(&attachment.session, request())
        .now_or_never()
        .expect("no mailbox hop")
        .unwrap();
    let View::Box { layout, .. } = view else {
        panic!("Box")
    };
    for (key, expected) in [("x", true), ("y", false)] {
        let mut action = layout.on_key.clone().unwrap();
        action.args["value"] = json!({"key":key});
        assert!(
            matches!(command(&mut attachment, Input::Action { action }).await, IntentOutcome::Applied { result } if result == json!({"handled":expected}))
        );
    }
    host.shutdown().await;
}
#[tokio::test]
async fn javascript_key_handler_preserves_handled_false() {
    let home = tempfile::tempdir().unwrap();
    let source = "export function register(on) { on('ui.render', {component:'Toast'}, () => h('Box', {onKey: e => ({handled:e.key === 'x'})}, h('Text',{},'Keys'))); }";
    let (host, mut attachment) = start(home.path(), Some(source), false).await;
    let View::Box { layout, .. } = host.render(&attachment.session, request()).await.unwrap()
    else {
        panic!("Box")
    };
    for (key, expected) in [("x", true), ("y", false)] {
        let mut action = layout.on_key.clone().unwrap();
        action.args["value"] = json!({"key":key});
        let outcome = command(&mut attachment, Input::Action { action }).await;
        assert!(
            matches!(outcome, IntentOutcome::Applied { result } if result == json!({"handled":expected}))
        );
    }
    host.shutdown().await;
}

#[tokio::test]
async fn an_outside_mod_wraps_the_question_builtin_with_next() {
    let home = tempfile::tempdir().unwrap();
    let source = "export function register(on) { on('ui.render', {component:'Question'}, async ($, e, next) => { const tree = await next(e); return {...tree, children:[h('Text',{},'Ask frame'), ...tree.children]}; }); }";
    let (host, attachment) = start(home.path(), Some(source), false).await;
    let request: render::RenderRequest = serde_json::from_value(json!({"component":"Question","requestId":"int_fixture","surface":"terminal","props":{"interactionId":"int_fixture","tool":"AskUserQuestion","questions":[{"question":"Which?","options":[{"id":"a","label":"Alpha"}],"freeText":true,"multi":false}]},"viewport":{"columns":80,"rows":24}})).unwrap();
    let tree = host.render(&attachment.session, request).await.unwrap();
    assert!(tree.fold().contains("Ask frame"));
    assert!(tree.fold().contains("Alpha"));
    assert!(matches!(tree, View::Box { layout, .. } if layout.on_key.is_some()));
    host.shutdown().await;
}

#[tokio::test]
async fn the_ask_handler_answers_the_live_question_through_the_existing_port() {
    let home = tempfile::tempdir().unwrap();
    let (host, mut attachment) = start(home.path(), None, false).await;
    let asking = {
        let host = host.clone();
        let session = attachment.session.clone();
        tokio::spawn(async move {
            host.ask(&session, serde_json::from_value(json!({"kind":"question","question":"Which?","options":[{"id":"a","label":"Alpha"},{"id":"b","label":"Beta"}]})).unwrap(), vec![AnswerSpec::Choice, AnswerSpec::Cancel]).await
        })
    };
    let open = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(frame) = attachment.events.next().await {
            if let Event::InteractionOpened { interaction } = frame.event {
                return interaction;
            }
        }
        panic!("question never opened")
    })
    .await
    .unwrap();
    while open
        .guard_until
        .is_some_and(|until| jiff::Timestamp::now() < until)
    {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let InteractionKind::Question(question) = &open.kind else {
        panic!("question")
    };
    let request: render::RenderRequest = serde_json::from_value(json!({"component":"Question","requestId":open.id,"surface":"terminal","props":{"interactionId":open.id,"tool":"AskUserQuestion","questions":[question]},"viewport":{"columns":80,"rows":24}})).unwrap();
    let View::Box { layout, .. } = host.render(&attachment.session, request).await.unwrap() else {
        panic!("Box")
    };
    let mut action = layout.on_key.clone().unwrap();
    action.args["value"] = json!({"key":"2"});
    let outcome = command(&mut attachment, Input::Action { action }).await;
    assert!(matches!(outcome, IntentOutcome::Applied { result } if result["handled"] == true));
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(10), asking)
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        Answer::Choice {
            ids: vec!["b".into()],
            other: None
        }
    );
    host.shutdown().await;
}
