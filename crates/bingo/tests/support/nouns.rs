//! The same author module exercises every host noun under both runtimes.
use bingo_core::{Host, HostConfig};
use bingo_mods::ModsPlugin;
use bingo_plugin_rpc::PluginRpcPlugin;
use bingo_sdk::*;
use futures::StreamExt;
use serde_json::{Value, json};
use std::path::Path;

fn install(home: &Path, node: bool) -> std::path::PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = home.join("fixture");
    std::fs::create_dir(&root).unwrap();
    std::fs::copy(
        workspace.join("crates/bingo-mods/tests/mods/nouns/parity.ts"),
        root.join("register.ts"),
    )
    .unwrap();
    let uses = [
        "session", "turn", "prompt", "tool", "command", "config", "model", "state", "store",
        "clock",
    ];
    let mut manifest = json!({"name":"fixture","version":"1","uses":uses});
    if node {
        super::copy(&workspace.join("sdk/typescript"), &home.join("sdk"));
        std::fs::write(root.join("main.ts"),"import {plugin} from '../sdk/src/plugin.ts'; await plugin(new URL('./register.ts',import.meta.url));").unwrap();
        manifest["entry"] = json!({"command":"node","args":["${PLUGIN_ROOT}/main.ts"]});
    } else {
        manifest["module"] = json!("register.ts");
    }
    std::fs::write(root.join("plugin.json"), manifest.to_string()).unwrap();
    root
}
async fn run(node: bool) -> Value {
    let home = tempfile::tempdir().unwrap();
    let root = install(home.path(), node);
    let mut config = HostConfig::new(Env::rooted(home.path())).with_layer(
        "user",
        json!({
            "pluginDirs":[root],"pluginTrust":{"fixture":{"tier":"rewrite","uses":["model"]}}
        }),
    );
    config.layers[0] = config.layers[0].clone().user();
    let mut plugins: Vec<Box<dyn Plugin>> = vec![Box::new(
        ModsPlugin::default()
            .with_dirs(vec![root.clone()])
            .with_project(home.path().into()),
    )];
    if node {
        plugins.push(Box::new(
            PluginRpcPlugin::default().with_project(home.path().into()),
        ));
    }
    let host = Host::build(plugins, config).await.unwrap();
    let mut session = host
        .open(
            SessionSelector::Create {
                spec: SessionSpec {
                    cwd: home.path().into(),
                    driver: Driver::Log,
                    ..Default::default()
                },
            },
            ClientIdentity {
                name: "fixture".into(),
                surface: "test".into(),
            },
            OpenOptions::default(),
        )
        .await
        .unwrap();
    let mut answer = loop {
        let frame = tokio::time::timeout(std::time::Duration::from_secs(15), session.events.next())
            .await
            .expect("noun fixture did not answer")
            .unwrap();
        if let Event::Notice {
            ref code, ref text, ..
        } = frame.event
        {
            assert_ne!(code, "INTERCEPTOR_SKIPPED", "{text}");
        }
        if let Event::Extension { kind, payload, .. } = frame.event
            && kind == "nouns"
        {
            break payload;
        }
    };
    assert_eq!(answer["plugin"], json!({"name":"fixture","root":root}));
    answer["plugin"]["root"] = json!("<fixture>");
    host.shutdown().await;
    answer
}
#[tokio::test]
async fn the_host_nouns_fixture_agrees_under_quickjs_and_node() {
    if !super::node_available() {
        return;
    }
    let native = run(false).await;
    let node = run(true).await;
    assert_eq!(native, node);
    assert_eq!(
        native,
        json!({"session":true,"prompt":{"queue":[],"last":null},"tools":true,"commands":true,
        "config":null,"model":"no model","state":{"version":1,"value":1},"store":{"version":1,"value":2},
        "clock":"number","plugin":{"name":"fixture","root":"<fixture>"},"frozen":true,"abort":null})
    );
}

#[tokio::test]
async fn node_registered_commands_refresh_callbacks_before_the_next_call() {
    if !super::node_available() {
        return;
    }
    let home = tempfile::tempdir().unwrap();
    let root = install(home.path(), true);
    let source = r#"export function register(on) {
      on('session.start', async ($) => {
        await $.command.register({name:'hello',hint:'fixture',args:{kind:'none'},family:'fixture'}, async () => ({kind:'applied',message:'first'}));
        await $.session.append({key:'ready',value:true});
      });
    }"#;
    std::fs::write(root.join("register.ts"), source).unwrap();
    let host = Host::build(
        vec![
            Box::new(ModsPlugin::default()),
            Box::new(PluginRpcPlugin::default().with_project(home.path().into())),
        ],
        HostConfig::new(Env::rooted(home.path()))
            .with_layer("fixture", json!({"pluginDirs":[root]})),
    )
    .await
    .unwrap();
    let mut session = host
        .open(
            SessionSelector::Create {
                spec: SessionSpec {
                    cwd: home.path().into(),
                    driver: Driver::Log,
                    ..Default::default()
                },
            },
            ClientIdentity {
                name: "fixture".into(),
                surface: "test".into(),
            },
            OpenOptions::default(),
        )
        .await
        .unwrap();
    async fn next(session: &mut Attachment) -> Event {
        tokio::time::timeout(std::time::Duration::from_secs(15), session.events.next())
            .await
            .expect("callback event")
            .unwrap()
            .event
    }
    loop {
        if matches!(next(&mut session).await,Event::Extension{ref kind,..} if kind == "ready") {
            break;
        }
    }
    for expected in ["first", "second"] {
        if expected == "second" {
            std::fs::write(root.join("register.ts"), source.replace("first", "second")).unwrap();
        }
        let intent = IntentId::mint();
        session.handle.submit(
            intent.clone(),
            Input::Action {
                action: Action {
                    name: "hello".into(),
                    args: Value::Null,
                },
            },
        );
        loop {
            if let Event::IntentAck {
                intent: ack,
                outcome,
            } = next(&mut session).await
                && ack == intent
            {
                assert!(
                    serde_json::to_value(outcome)
                        .unwrap()
                        .to_string()
                        .contains(expected)
                );
                break;
            }
        }
    }
    host.shutdown().await;
}
