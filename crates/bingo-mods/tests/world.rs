#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use serde_json::json;
use support::Fixture;

#[tokio::test]
async fn filesystem_face_reads_a_scoped_file_through_the_module() {
    let fixture = Fixture::new(include_str!("mods/world/files.ts"), &["fs"], json!({})).await;
    let path = fixture.home.path().join("sample.txt");
    std::fs::write(&path, "world").unwrap();
    let answer = fixture.call("world", "echo", json!({"path":path})).await;
    assert_eq!(answer, json!({"value":"world"}));
    fixture.host.shutdown().await;
}

async fn call(
    fixture: &Fixture,
    noun: &str,
    method: &str,
    input: serde_json::Value,
) -> serde_json::Value {
    let name = format!("{noun}.{method}");
    let before = fixture.call("world", "echo", json!({"counts":true})).await[&name]
        .as_u64()
        .unwrap_or_default();
    let answer = fixture
        .call(
            "world",
            "echo",
            json!({"noun":noun,"method":method,"input":input,"session":fixture.attachment.session}),
        )
        .await;
    assert!(answer.get("error").is_none(), "{noun}.{method}: {answer}");
    let after = fixture.call("world", "echo", json!({"counts":true})).await[&name]
        .as_u64()
        .unwrap_or_default();
    assert_eq!(after, before + 1, "{name} must raise exactly one chain");
    answer["value"].clone()
}
async fn world() -> Fixture {
    Fixture::new(
        include_str!("mods/world/call.ts"),
        &["fs", "http", "process", "env"],
        json!({}),
    )
    .await
}
#[tokio::test]
async fn filesystem_methods_are_bounded_and_support_binary_data() {
    let fixture = world().await;
    let data = fixture.home.path().join(".bingo/data/plugins/fixture/data");
    let file = data.join("binary");
    call(
        &fixture,
        "fs",
        "write",
        json!({"path":file,"body":"/wA=","encoding":"base64"}),
    )
    .await;
    assert_eq!(
        call(
            &fixture,
            "fs",
            "read",
            json!({"path":file,"encoding":"base64"})
        )
        .await,
        "/wA="
    );
    assert_eq!(
        call(&fixture, "fs", "exists", json!({"path":file})).await,
        true
    );
    assert_eq!(
        call(&fixture, "fs", "stat", json!({"path":file})).await["size"],
        2
    );
    assert_eq!(
        call(&fixture, "fs", "list", json!({"path":data,"limit":1})).await,
        json!(["binary"])
    );
    std::fs::write(fixture.home.path().join("marker"), "").unwrap();
    assert_eq!(
        call(&fixture, "fs", "ancestors", json!({"name":"marker"})).await,
        json!([dunce::canonicalize(fixture.home.path()).unwrap()])
    );
    std::fs::write(&file, vec![0; 4 * 1024 * 1024 + 1]).unwrap();
    let answer = fixture.call("world","echo",json!({"noun":"fs","method":"read","input":{"path":file},"session":fixture.attachment.session})).await;
    let error = answer["error"].as_str().unwrap();
    assert!(
        error.contains("4194305") && error.contains("4194304"),
        "{error}"
    );
    fixture.host.shutdown().await;
}
#[tokio::test]
async fn environment_overlay_reaches_only_its_child() {
    let fixture = world().await;
    let name = format!("BINGO_WORLD_{}", std::process::id());
    let before = std::env::var(&name);
    call(
        &fixture,
        "env",
        "set",
        json!({"name":name,"value":"child-only"}),
    )
    .await;
    assert_eq!(
        call(&fixture, "env", "get", json!({"name":name})).await,
        json!(before.as_ref().ok())
    );
    let shell = bingo_process::shell::resolve().unwrap();
    let output = call(
        &fixture,
        "process",
        "run",
        json!({"argv":[shell.path(),"-c",format!("printf '%s' \"${name}\"")]}),
    )
    .await;
    assert_eq!(output["stdout"], "child-only");
    assert_eq!(output["code"], 0);
    assert_eq!(std::env::var(&name), before);
    fixture.host.shutdown().await;
}
#[tokio::test]
async fn spawned_process_streams_are_owned_and_reload_closes_the_tree() {
    let fixture = world().await;
    let shell = bingo_process::shell::resolve().unwrap();
    let id = call(
        &fixture,
        "process",
        "spawn",
        json!({"argv":[shell.path(),"-c","cat"]}),
    )
    .await;
    call(
        &fixture,
        "process",
        "write",
        json!({"id":id,"body":"stream"}),
    )
    .await;
    call(&fixture, "process", "write", json!({"id":id,"body":null})).await;
    assert_eq!(
        call(&fixture, "process", "read", json!({"id":id})).await["stdout"],
        "stream"
    );
    call(&fixture, "process", "kill", json!({"id":id})).await;
    let id = call(
        &fixture,
        "process",
        "spawn",
        json!({"argv":[shell.path(),"-c","sleep 60 & echo ready; wait"]}),
    )
    .await;
    let runtime = fixture.plugin.clone();
    runtime.reload(&fixture.root).await.unwrap();
    let answer = fixture.call("world","echo",json!({"noun":"process","method":"kill","session":fixture.attachment.session,"input":{"id":id}})).await;
    assert!(answer["error"].as_str().unwrap().contains("not owned"));
    fixture.host.shutdown().await;
}

#[tokio::test]
async fn a_granted_noun_posts_one_notice_per_session() {
    let mut fixture = world().await;
    call(&fixture, "env", "get", json!({"name":"PATH"})).await;
    call(&fixture, "env", "get", json!({"name":"PATH"})).await;
    call(
        &fixture,
        "env",
        "set",
        json!({"name":"BINGO_NOTICE_TEST","value":"local"}),
    )
    .await;
    let notices = fixture.notices().await;
    assert_eq!(
        notices
            .iter()
            .filter(|text| text.contains("uses the granted env noun"))
            .count(),
        1,
        "{notices:?}"
    );
    fixture.host.shutdown().await;
}
#[tokio::test]
async fn esc_drops_a_pending_http_request() {
    use bingo_sdk::*;
    use futures::StreamExt;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let source = format!(
        "export function register(on) {{ on('turn.step', async ($,e,next) => {{ await $.http.fetch({{url:{}}}); return next(e); }}); }}",
        json!(url)
    );
    let mut fixture = Fixture::agent(&source, &["http"], json!({})).await;
    fixture
        .attachment
        .handle
        .submit(IntentId::mint(), Input::text("go", Origin::surface("test")));
    let (mut socket, _) =
        tokio::time::timeout(std::time::Duration::from_secs(10), listener.accept())
            .await
            .unwrap()
            .unwrap();
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        head.push(socket.read_u8().await.unwrap());
    }
    fixture
        .attachment
        .handle
        .interrupt(IntentId::mint(), InterruptScope::Head);
    let status = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(frame) = fixture.attachment.events.next().await {
            if let Event::TurnCompleted { status, .. } = frame.event {
                return status;
            }
        }
        panic!("session ended");
    })
    .await
    .unwrap();
    assert!(matches!(status, TurnStatus::Interrupted { .. }));
    let mut tail = Vec::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        socket.read_to_end(&mut tail),
    )
    .await
    .expect("esc closes fetch")
    .unwrap();
    let _ = socket.shutdown().await;
    fixture.host.shutdown().await;
}

#[tokio::test]
async fn project_world_calls_need_user_layer_grants() {
    use bingo_core::{Host, HostConfig};
    use bingo_mods::ModsPlugin;
    use bingo_sdk::*;
    use std::sync::Arc;
    struct Echo;
    #[async_trait::async_trait]
    impl WireService for Echo {
        fn face(&self) -> Option<Face> {
            Some(
                Face::new("testworld")
                    .method::<serde_json::Value, serde_json::Value>("check", false),
            )
        }
        async fn call(
            &self,
            _: &str,
            value: serde_json::Value,
        ) -> Result<serde_json::Value, ServiceError> {
            Ok(value)
        }
    }
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let plugin = project.path().join(".bingo/plugins/project-world");
    std::fs::create_dir_all(&plugin).unwrap();
    std::fs::write(
        plugin.join("plugin.json"),
        r#"{"name":"project-world","version":"1","module":"main.ts","uses":["env","fs","state"]}"#,
    )
    .unwrap();
    std::fs::write(plugin.join("main.ts"),"export function register(on) { on('testworld.check',async($,e,next)=>{ for(const noun of ['env','fs']) { let value='granted'; try { if(noun==='env') await $.env.set({session:e.session,name:'BINGO_PROJECT_TEST',value:'overlay'}); else await $.fs.write({session:e.session,path:'outside-data.txt',body:'granted'}); } catch(error) {value=String(error);} await $.store.set(noun,value); } return next(e); }); }").unwrap();
    // State is global only so this test can inspect the result outside the hook's synthetic host site.
    let mut manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(plugin.join("plugin.json")).unwrap())
            .unwrap();
    manifest["uses"] = json!(["env", "fs", "store"]);
    std::fs::write(plugin.join("plugin.json"), manifest.to_string()).unwrap();
    for trusted in [false, true] {
        let mut config = HostConfig::new(Env::rooted(home.path()));
        if trusted {
            config = config.with_layer(
                "user",
                json!({"pluginTrust":{"project-world":{"tier":"rewrite","uses":["env","fs"]}}}),
            );
            let user = config.layers.pop().unwrap().user();
            config.layers.push(user);
        }
        let host = Host::build(
            vec![Box::new(
                ModsPlugin::default().with_project(project.path().into()),
            )],
            config,
        )
        .await
        .unwrap();
        bingo_sdk::render::under_input(
            true,
            host.approve(Approval {
                root: project.path().into(),
                what: "plugin:project-world".into(),
            }),
        )
        .await
        .unwrap();
        for source in &host.registry().sources.plugins {
            source.reload().await.unwrap();
        }
        host.open_service("testworld", Arc::new(Echo)).unwrap();
        let session = host
            .open(
                SessionSelector::Create {
                    spec: SessionSpec {
                        cwd: project.path().into(),
                        driver: Driver::Log,
                        ..Default::default()
                    },
                },
                ClientIdentity {
                    name: "test".into(),
                    surface: "test".into(),
                },
                OpenOptions::default(),
            )
            .await
            .unwrap();
        host.service_wire("testworld")
            .unwrap()
            .call("check", json!({"session":session.session}))
            .await
            .unwrap();
        let store = bingo_plugin_host::state::Store::new(
            &Env::rooted(home.path()).data_dir,
            "project-world",
        );
        for noun in ["env", "fs"] {
            let value = store
                .get("global".into(), noun.into())
                .await
                .unwrap()
                .unwrap()
                .value;
            if trusted {
                assert_eq!(value, "granted");
            } else {
                assert!(
                    value
                        .as_str()
                        .unwrap()
                        .contains("Grant it under pluginTrust"),
                    "{value}"
                );
            }
        }
        host.shutdown().await;
    }
}
