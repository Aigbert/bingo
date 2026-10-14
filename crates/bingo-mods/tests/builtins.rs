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
        Box::new(bingo_mod_palette::Palette),
        Box::new(bingo_mod_sessions::Sessions),
        Box::new(bingo_mod_plugins::Plugins),
    ];
    if let Some(source) = source {
        let root = home.join("outside");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(
            root.join("plugin.json"),
            json!({"name":"outside","version":"1","module":"main.ts","uses":["ui","command","session"]}).to_string(),
        )
        .unwrap();
        std::fs::write(root.join("main.ts"), source).unwrap();
        plugins.push(Box::new(
            bingo_mods::ModsPlugin::default()
                .with_dirs(vec![root])
                .with_project(home.into()),
        ));
    }
    let env = Env::rooted(home);
    for name in [
        "hello",
        "ask",
        "bingo.mod.palette",
        "bingo.mod.sessions",
        "bingo.mod.plugins",
    ] {
        bingo_core::plugins::switch(&env, name, !disabled).unwrap();
    }
    let path = bingo_core::settings::user_path(&env);
    let mut config = HostConfig::new(env);
    config.layers.push(bingo_core::settings::Layer::new(
        bingo_core::settings::LayerOrigin::User,
        path.display().to_string(),
        bingo_core::settings::read_document(&path).unwrap(),
    ));
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
async fn the_ask_handler_returns_an_answer_for_the_surface_to_send() {
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
    let IntentOutcome::Applied { result } = outcome else {
        panic!("applied")
    };
    assert_eq!(result["handled"], true);
    assert!(
        !asking.is_finished(),
        "the mod cannot resolve the interaction"
    );
    attachment.handle.answer(
        IntentId::mint(),
        open.id.clone(),
        serde_json::from_value(result["answer"].clone()).unwrap(),
        Activation::Keyboard,
    );
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

async fn palette_request(host: &Host, attachment: &Attachment) -> render::RenderRequest {
    let catalog = host.catalog(CatalogKind::Commands).await.unwrap();
    let specs = catalog
        .entries
        .iter()
        .filter_map(|entry| serde_json::from_value(entry.meta.clone()).ok())
        .collect::<Vec<_>>();
    let props = component::PaletteProps {
        query: "".into(),
        open: component::PaletteOpen::Keys,
        commands: catalog.entries,
        bindings: keys::bindings(&specs, &Default::default(), &[]).commands,
        key_hints: Default::default(),
        sessions: vec![],
        session: attachment.snapshot.summary.id.clone(),
        conversation: vec![],
        action: Action {
            name: "surface:palette".into(),
            args: json!({"handler":"palette"}),
        },
        catalogs: Default::default(),
    };
    render::RenderRequest {
        component: render::Component::Palette,
        request_id: "palette".into(),
        surface: render::SurfaceName::Terminal,
        props: serde_json::to_value(props).unwrap(),
        viewport: render::Viewport {
            columns: 80,
            rows: 12,
            place: None,
        },
    }
}
#[tokio::test]
async fn a_js_command_and_its_keys_appear_inside_a_wrapped_palette() {
    let home = tempfile::tempdir().unwrap();
    let source = r#"export function register(on) {
        on('session.start', async ($) => {
            await $.command.register({name:'sample',hint:'sample command',keys:['alt+z'],args:{kind:'none'},family:'fixture'}, async ($) => ({kind:'applied',message:'sample ran'}));
            await $.session.append({key:'palette-ready',value:true});
        });
        on('ui.render',{component:'Palette'},async ($,e,next)=>{const tree=await next(e);return {...tree,children:[h('Text',{},'Outside frame'),...tree.children]};});
    }"#;
    let (host, mut attachment) = start(home.path(), Some(source), false).await;
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        while let Some(frame) = attachment.events.next().await {
            if matches!(frame.event,Event::Extension{kind,..} if kind=="palette-ready") {
                return;
            }
        }
        panic!("registration ended");
    })
    .await
    .unwrap();
    let entries = host.catalog(CatalogKind::Plugins).await.unwrap().entries;
    assert!(entries.iter().any(|entry| {
        entry.meta["commands"]
            .as_array()
            .is_some_and(|commands| commands.iter().any(|command| command["name"] == "sample"))
    }));
    let mut request = palette_request(&host, &attachment).await;
    let mut schema = json!({"leaf":"choice metadata"});
    for _ in 0..6 {
        schema = json!({"properties":{"nested":schema}});
    }
    request.props["catalogs"]["plugins"] = json!([{"id":"demo", "label":"demo", "meta":schema}]);
    let tree = host.render(&attachment.session, request).await.unwrap();
    assert!(tree.fold().contains("Outside frame"));
    let payload = serde_json::to_value(&tree).unwrap();
    assert_eq!(
        payload["onKey"]["args"]["props"]["catalogs"]["plugins"][0]["id"],
        "demo"
    );
    assert!(payload["onKey"]["args"]["props"]["catalogs"]["plugins"][0]["meta"].is_null());
    assert!(tree.fold().contains("alt+z"));
    assert!(tree.fold().contains("sample command"), "{}", tree.fold());
    assert!(matches!(tree,View::Box{layout,..} if layout.on_key.is_some()));
    host.shutdown().await;
}
#[tokio::test]
async fn disabled_palette_has_no_commands_or_render_override() {
    let home = tempfile::tempdir().unwrap();
    let (host, attachment) = start(home.path(), None, true).await;
    let request = palette_request(&host, &attachment).await;
    assert!(
        !request.props["commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["id"] == "help" || entry["id"] == "resume")
    );
    assert_eq!(
        host.render(&attachment.session, request.clone())
            .await
            .unwrap(),
        request.engine()
    );
    host.shutdown().await;
}

#[tokio::test]
async fn live_switch_restores_commands_bindings_and_drawing_even_when_booted_off() {
    for disabled in [false, true] {
        let home = tempfile::tempdir().unwrap();
        let (host, mut attachment) = start(home.path(), None, disabled).await;
        for enabled in [true, false, true] {
            let verb = if enabled { "enable" } else { "disable" };
            let ack = command(
                &mut attachment,
                Input::text(format!("/plugins {verb} hello"), Origin::surface("test")),
            )
            .await;
            assert!(matches!(ack, IntentOutcome::Applied { .. }), "{ack:?}");
            let commands = host.catalog(CatalogKind::Commands).await.unwrap();
            let specs: Vec<CommandSpec> = commands
                .entries
                .into_iter()
                .map(|e| serde_json::from_value(e.meta).unwrap())
                .collect();
            let bindings = bingo_sdk::keys::bindings(&specs, &Default::default(), &[]);
            assert_eq!(bindings.commands.contains_key("ctrl+x"), enabled);
            let tree = host.render(&attachment.session, request()).await.unwrap();
            assert_eq!(tree.fold().contains("Hello from a built-in mod"), enabled);
            let ack = command(
                &mut attachment,
                Input::text("/hello", Origin::surface("test")),
            )
            .await;
            assert_eq!(
                matches!(ack, IntentOutcome::Applied { .. }),
                enabled,
                "{ack:?}"
            );
            let plugins = host.catalog(CatalogKind::Plugins).await.unwrap();
            assert_eq!(
                plugins
                    .entries
                    .iter()
                    .find(|e| e.id == "hello")
                    .unwrap()
                    .meta["state"]["kind"],
                if enabled { "on" } else { "off" }
            );
        }
        for link in &host.registry().links {
            assert!(
                host.registry()
                    .plugins
                    .iter()
                    .any(|plugin| plugin.id == link.plugin)
            );
        }
        host.shutdown().await;
    }
}

#[tokio::test]
async fn sessions_obeys_the_same_switch_for_commands_and_rendering() {
    for disabled in [false, true] {
        let home = tempfile::tempdir().unwrap();
        let (host, attachment) = start(home.path(), None, disabled).await;
        let commands = host.catalog(CatalogKind::Commands).await.unwrap();
        assert_eq!(
            commands.entries.iter().any(|entry| entry.id == "sessions"),
            !disabled
        );
        let request: render::RenderRequest = serde_json::from_value(json!({
            "component":"Sessions", "requestId":"sessions", "surface":"terminal",
            "viewport":{"columns":80,"rows":20},
            "props":{"session":attachment.session,"cwd":home.path(),
                "query":"","scope":"here","limit":50,"sessions":[],"now":"2026-10-15T12:00:00Z",
                "action":{"name":"surface:sessions","args":{}}}
        }))
        .unwrap();
        let view = host
            .render(&attachment.session, request.clone())
            .await
            .unwrap();
        assert_eq!(view == request.engine(), disabled);
        host.shutdown().await;
    }
}

#[tokio::test]
async fn outside_mod_wraps_the_plugins_site_and_disabled_commands_keep_tables() {
    let home = tempfile::tempdir().unwrap();
    let source = r#"export function register(on) {
        on('ui.render',{component:'Plugins'},async ($,e,next)=>{const tree=await next(e);return {...tree,children:[h('Text',{},'Outside plugins frame'),...tree.children]};});
    }"#;
    let (host, mut attachment) = start(home.path(), Some(source), false).await;
    let props = component::PluginsProps {
        open: component::PluginsOpen::Plugins,
        session: attachment.session.clone(),
        plugins: host.catalog(CatalogKind::Plugins).await.unwrap().entries,
        action: Action {
            name: "surface:plugins".into(),
            args: json!({"handler":"panel"}),
        },
    };
    let tree = host
        .render(
            &attachment.session,
            render::RenderRequest {
                component: render::Component::Plugins,
                request_id: "panel".into(),
                surface: render::SurfaceName::Terminal,
                props: serde_json::to_value(props).unwrap(),
                viewport: render::Viewport {
                    place: None,
                    columns: 108,
                    rows: 18,
                },
            },
        )
        .await
        .unwrap();
    assert!(
        tree.fold().contains("Outside plugins frame"),
        "{}",
        tree.fold()
    );
    assert!(tree.fold().contains("one list"));
    let opened = command(
        &mut attachment,
        Input::text("/modules", Origin::surface("test")),
    )
    .await;
    assert!(
        matches!(opened,IntentOutcome::Applied {result} if result["surface"]["name"]=="Plugins")
    );
    command(
        &mut attachment,
        Input::text(
            "/plugins disable bingo.mod.plugins",
            Origin::surface("test"),
        ),
    )
    .await;
    let plain = command(
        &mut attachment,
        Input::text("/plugins", Origin::surface("test")),
    )
    .await;
    assert!(
        matches!(plain,IntentOutcome::Applied {ref result} if result["view"]["kind"]=="table"),
        "{plain:?}"
    );
    host.shutdown().await;
}
