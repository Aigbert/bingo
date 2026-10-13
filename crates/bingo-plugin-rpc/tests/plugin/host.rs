use super::harness::*;
use bingo_sdk::{Delivery, Input, testing::HostCall};
use serde_json::{Value, json};

async fn reverse(started: &Started, method: &str, params: Value) -> Value {
    let tool = only_tool(&started.manager).await;
    let (_, result) = call(
        &tool,
        json!({"host":{"method":method,"params":params}}),
        started.project.path(),
    )
    .await;
    serde_json::from_str(&said(&result.unwrap())).unwrap()
}

#[tokio::test]
async fn reverse_verbs_bind_the_process_identity_and_do_not_wait_for_a_turn() {
    let started = started_with(&[("stub", &[])]).await;
    started
        .listener
        .host
        .on_signal(|_, _, _, _| async { Ok(()) });
    started
        .listener
        .host
        .on_extend(|_, _, _, _| async { Ok(()) });
    started
        .listener
        .host
        .on_deliver(|_, _, _, _| async { Ok(()) });
    let slot = json!({"kind":"status","text":"Ready","tone":"neutral","until":null});
    let place = reverse(
        &started,
        "host/place",
        json!({"session":"ses_1","slot":"status","view":slot}),
    )
    .await;
    assert!(place.get("error").is_none(), "{place}");
    assert_eq!(place["result"], Value::Null);
    let extend = reverse(
        &started,
        "host/extend",
        json!({"session":"ses_1","key":"record","value":42}),
    )
    .await;
    assert!(extend.get("error").is_none(), "{extend}");
    let delivered = reverse(&started, "host/deliver", json!({"session":"ses_1","input":{"kind":"text","text":"hello","origin":{"surface":"forged"}},"wake":false})).await;
    assert!(delivered.get("error").is_none(), "{delivered}");
    let calls = started.listener.host.calls();
    assert!(calls.iter().any(|call| matches!(call, HostCall::Signal { plugin, payload, .. } if plugin == "stub" && payload["slots"][0] == slot)));
    assert!(calls.iter().any(|call| matches!(call, HostCall::Extend { plugin, kind, payload, .. } if plugin == "stub" && kind == "record" && payload == &json!(42))));
    assert!(calls.iter().any(|call| matches!(call, HostCall::Deliver { input: Input::Text { origin, .. }, delivery: Delivery::Hold, .. } if origin.surface == "plugin:stub")));
    let unknown = reverse(&started, "future/verb", json!({})).await;
    assert_eq!(unknown["error"]["code"], -32601);
    started.manager.shutdown().await;
}

#[tokio::test]
async fn state_versions_refuse_stale_writes_and_survive_a_restart() {
    let started = started_with(&[("stub", &[])]).await;
    let get = json!({"scope":"global","key":"greeting"});
    let missing = reverse(&started, "host/state/get", get.clone()).await;
    assert!(missing.get("error").is_none(), "{missing}");
    assert_eq!(missing["result"], Value::Null);
    let set = json!({"scope":"global","key":"greeting","value":"hello"});
    let first = reverse(&started, "host/state/set", set.clone()).await;
    assert_eq!(first["result"], json!({"version":1,"value":"hello"}));
    let mut stale = set;
    stale["ifVersion"] = json!(9);
    let conflict = reverse(&started, "host/state/set", stale).await;
    assert_eq!(conflict["error"]["data"]["version"], 1);
    started.manager.shutdown().await;
    let manager = bingo_plugin_rpc::Manager::new(
        bingo_sdk::Env::rooted(started.home.path()),
        Default::default(),
        Default::default(),
    );
    manager
        .start(started.project.path(), started.host.clone())
        .await;
    let tool = only_tool(&manager).await;
    let (_, result) = call(
        &tool,
        json!({"host":{"method":"host/state/get","params":get}}),
        started.project.path(),
    )
    .await;
    let value: Value = serde_json::from_str(&said(&result.unwrap())).unwrap();
    assert_eq!(value["result"], first["result"]);
    manager.shutdown().await;
}

#[tokio::test]
async fn contribution_notifications_replace_the_whole_list() {
    let started = started_with(&[("stub", &[])]).await;
    let tool = only_tool(&started.manager).await;
    let (_, result) = call(
        &tool,
        json!({"announce":{"method":"tools/changed","params":{"tools":[]}}}),
        started.project.path(),
    )
    .await;
    result.unwrap();
    assert!(started.manager.tools().await.is_empty());
    let (_, result) = call(
        &tool,
        json!({"announce":{"method":"commands/changed","params":{"commands":[]}}}),
        started.project.path(),
    )
    .await;
    result.unwrap();
    assert!(started.manager.commands().await.is_empty());
    started.manager.shutdown().await;
}

#[tokio::test]
async fn model_completion_reaches_the_hosts_default_refusal() {
    let started = started_with(&[("stub", &[])]).await;
    let request = bingo_sdk::ModelRequest {
        model: "ignored".into(),
        max_tokens: 100,
        system: Vec::new(),
        messages: Vec::new(),
        tools: Vec::new(),
        reasoning: None,
        session: None,
        provider_options: Default::default(),
    };
    let answer = reverse(
        &started,
        "host/model/complete",
        json!({"session":"ses_1","request":request}),
    )
    .await;
    assert_eq!(answer["error"]["code"], -32603);
    assert!(
        answer["error"]["message"]
            .as_str()
            .unwrap()
            .contains("this host does not complete model requests"),
        "{answer}"
    );
    started.manager.shutdown().await;
}
