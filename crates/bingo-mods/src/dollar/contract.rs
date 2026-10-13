//! Exhaustive dispatch through the author's dollar, against M111's host.
use super::{
    Runtime,
    table::{Door, ROWS, Row},
};
use crate::engine::{Engine, Limits};
use bingo_plugin_host::{callback, ops, state::Store};
use bingo_sdk::*;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};

pub(super) fn sample(name: &str, session: &SessionId) -> Value {
    let input = Input::text("seed", Origin::surface("fixture"));
    match name {
        "session.messages" => json!({"session":session,"page":HistoryPage::default()}),
        "session.send" => json!({"to":session,"input":input,"delivery":"hold"}),
        "session.append" => json!({"session":session,"key":"sample","value":1}),
        "session.authorize" => {
            json!({"session":session,"kind":InteractionKind::Confirm {title:"fixture".into(),detail:"fixture".into()},"answers":["confirm","cancel"]})
        }
        "prompt.submit" => json!({"session":session,"input":input}),
        "tool.call" => json!({"session":session,"call":{"callId":"c","name":"Echo","input":{}}}),
        "tool.register" => {
            json!({"session":session,"spec":{"name":"extra","description":"fixture","inputSchema":{"type":"object"}},"callback":"tool/extra"})
        }
        "command.register" => {
            json!({"session":session,"spec":{"name":"extra","hint":"fixture","args":{"kind":"none"},"family":"fixture"},"callback":"command/extra"})
        }
        "command.run" => json!({"session":session,"name":"status","args":null}),
        "config.read" => json!({"session":session,"key":"thinking"}),
        "config.set" => json!({"session":session,"key":"thinking","value":"high"}),
        "model.complete" => {
            json!({"session":session,"request":{"model":"offline","maxTokens":100,"system":[],"messages":[],"tools":[]}})
        }
        "model.classify" => json!({"session":session,"text":"question","labels":["yes","no"]}),
        "model.fork" => json!({"session":session,"prompt":"side"}),
        "state.get" | "state.set" => json!({"scope":session,"key":"key","value":1}),
        "store.get" | "store.set" | "store.delete" => {
            json!({"scope":"global","key":"key","value":1})
        }
        "clock.sleep" => json!({"ms":0}),
        "clock.after" | "clock.every" => {
            json!({"session":session,"ms":3_600_000,"callback":"clock/1"})
        }
        "session.version" | "tool.list" | "command.list" | "store.keys" | "clock.now" => {
            Value::Null
        }
        _ => json!({"session":session}),
    }
}
struct Port;
#[async_trait::async_trait]
impl SessionPort for Port {
    fn submit(&self, _: IntentId, _: Input) {}
    fn interrupt(&self, _: IntentId, _: InterruptScope) {}
    fn answer(&self, _: IntentId, _: InteractionId, _: Answer, _: Activation) {}
    async fn history(&self, _: HistoryPage) -> Result<HistoryChunk, KernelError> {
        Ok(HistoryChunk {
            items: Vec::new(),
            next: None,
            generation: 0,
            oversized: None,
        })
    }
    async fn events_since(&self, _: Seq) -> Result<FrameStream, KernelError> {
        Ok(Box::pin(futures::stream::empty()))
    }
}
struct Callback;
#[async_trait::async_trait]
impl callback::Executor for Callback {
    fn stopped(&self) -> CancellationToken {
        CancellationToken::new()
    }
    async fn prepare(&self) -> Result<(), String> {
        Ok(())
    }
    async fn call(&self, _: &str, _: Value, _: &HookContext) -> Result<Value, String> {
        Ok(Value::Null)
    }
}
fn host() -> testing::TestHost {
    let host = testing::TestHost::default();
    host.on_notice(|_, _, _| async { Ok(()) });
    host.on_catalog(|kind| async move {Ok(Catalog{kind,entries:if kind == CatalogKind::Plugins {vec![CatalogEntry{id:"fixture".into(),label:"fixture".into(),meta:json!({"enabled":true,"tier":"rewrite","uses":ROWS.iter().map(|row|row.noun).collect::<Vec<_>>()})}]}else{Vec::new()}})});
    host.on_open(|_,_,_| async {Ok(Attachment{session:SessionId::from_raw("s"),snapshot:SessionState::new(serde_json::from_value(json!({"id":"s","cwd":".","createdAt":"2026-01-01T00:00:00Z","updatedAt":"2026-01-01T00:00:00Z","usage":Usage::default()})).unwrap()),history:None,events:Box::pin(futures::stream::pending()),handle:SessionHandle(Arc::new(Port))})});
    host.on_complete(|_, _, _| async {
        Ok(vec![ModelEvent::TextDelta {
            id: "a".into(),
            delta: "yes".into(),
        }])
    });
    host.on_ask(|_, _, _| async { Ok(Answer::Confirm) });
    host.on_extend(|_, _, _, _| async { Ok(()) });
    host.on_deliver(|_, _, _, _| async { Ok(()) });
    host.on_reconfigure(|_, _| async { Ok(()) });
    host.on_invoke(|_, call| async {
        Ok(ToolOutcome {
            item: ItemId::mint(),
            call_id: call.call_id,
            output: ToolOutput::text("ok"),
            status: ItemStatus::Completed,
            duration_ms: 0,
        })
    });
    let registry = Arc::new(callback::Registry::default());
    registry.insert("fixture".into(), Arc::new(Callback));
    host.services()
        .add(
            callback::KEY.name().into(),
            service::Service {
                value: Arc::new(registry),
                wire: None,
            },
        )
        .unwrap();
    host
}
pub(super) async fn run(engine: &Engine, row: &Row, host: &HostHandle, state: &Store) -> Value {
    let name = format!("{}.{}", row.noun, row.method);
    let input = sample(&name, &SessionId::from_raw("s"));
    let mut call = engine.call("fixture.call",json!({"name":name,"input":input}),json!({"session":"s","cwd":".","origin":{"plugin":"fixture","tier":"rewrite"},"plugin":{"name":"fixture","root":"."}})).unwrap();
    loop {
        tokio::select! {
            biased;
            Some(operation) = call.operations.recv() => {
                let result = call.reads.scope(ops::call(host,"fixture",state,&operation.method,operation.params)).await;
                operation.reply.send(result).unwrap();
            }
            result = &mut call.result => return result.unwrap().unwrap_or_else(|error| panic!("{name}: {error}")),
        }
    }
}
#[tokio::test]
async fn host_rows_answer_on_the_test_host() {
    let host = host();
    let root = tempfile::tempdir().unwrap();
    let state = Store::new(root.path(), "fixture");
    let runtime =
        Runtime::install(&host.handle(), root.path().into(), Duration::from_secs(1)).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/mods/nouns");
    let engine = Engine::start(&source, "all.ts", json!({}), Limits::default())
        .await
        .unwrap();
    for row in ROWS.iter().filter(|row| !row.world()) {
        let answer = run(&engine, row, &host.handle(), &state).await;
        assert!(
            (row.accepts)(&answer),
            "{}.{}: {answer}",
            row.noun,
            row.method
        );
        if let Door::Kernel(entry) = row.door {
            assert_eq!(entry.name, format!("{}.{}", row.noun, row.method));
        }
    }
    engine.close().await;
    runtime.stop().await;
}
