//! Real kernel dispatch complements the scripted host's exhaustive answers.
use super::{contract::sample, table::ROWS};
use crate::ModsPlugin;
use bingo_core::{Host, HostConfig};
use bingo_sdk::*;
use futures::StreamExt;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Default)]
struct Counts(Mutex<BTreeMap<String, usize>>);
impl Counts {
    fn get(&self, name: &str) -> usize {
        *self.0.lock().unwrap().get(name).unwrap_or(&0)
    }
}
#[async_trait::async_trait]
impl Interceptor for Counts {
    fn id(&self) -> &str {
        "counts"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher::default()
    }
    async fn chain(
        &self,
        entry: &catalogue::Entry,
        _: &HookContext,
        input: Value,
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let driver = entry.name == "session.append"
            && matches!(
                input["kind"].as_str(),
                Some("dollar-fixture" | "dollar-answer")
            );
        let result = next.run(input).await;
        if !driver {
            *self
                .0
                .lock()
                .unwrap()
                .entry(entry.name.to_string())
                .or_default() += 1;
        }
        result
    }
}
struct Offline;
#[async_trait::async_trait]
impl Provider for Offline {
    fn id(&self) -> &str {
        "offline"
    }
    fn endpoint(&self, _: &str) -> EndpointCapabilities {
        EndpointCapabilities::default()
    }
    async fn stream(
        &self,
        request: ModelRequest,
        _: CancellationToken,
    ) -> Result<ModelStream, ProviderError> {
        if request.session.is_some()
            && serde_json::to_string(&request.messages)
                .unwrap()
                .contains("hold-model")
        {
            return Ok(Box::pin(futures::stream::pending()));
        }
        Ok(Box::pin(futures::stream::iter([
            Ok(ModelEvent::TextStart { id: "a".into() }),
            Ok(ModelEvent::TextDelta {
                id: "a".into(),
                delta: "yes".into(),
            }),
            Ok(ModelEvent::TextEnd { id: "a".into() }),
            Ok(ModelEvent::Finish {
                usage: Usage::default(),
                finish_reason: FinishReason::unified(UnifiedFinish::Stop),
            }),
        ])))
    }
}
struct Echo;
#[async_trait::async_trait]
impl Tool for Echo {
    fn spec(&self) -> ToolSpec {
        serde_json::from_value(json!({"name":"Echo","description":"fixture","inputSchema":{"type":"object"},"traits":{"readOnly":true}})).unwrap()
    }
    async fn call(&self, _: Value, _: &ToolContext) -> Result<ToolOutput, ToolError> {
        Ok(ToolOutput::text("echo"))
    }
}
struct Compact;
#[async_trait::async_trait]
impl Compactor for Compact {
    async fn compact(
        &self,
        _: CompactContext<'_>,
        _: CompactReason,
    ) -> Result<Compaction, CompactError> {
        Err(KernelError::new(ErrorCode::Internal, "fixture compactor").into())
    }
}
struct FixturePlugin(Arc<Counts>);
static MANIFEST: PluginManifest = PluginManifest {
    id: "offline",
    version: "1",
    uses: &[],
    provides: &[],
    requires: &[],
    config: None,
};
#[async_trait::async_trait]
impl Plugin for FixturePlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }
    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        registrar.provider(Arc::new(Offline));
        registrar.tool(Arc::new(Echo));
        registrar.add(Contribution::Compactor(Arc::new(Compact)));
        registrar.add(Contribution::Interceptor {
            external: None,
            link: self.0.clone(),
        });
        Ok(())
    }
}
async fn frame(session: &mut Attachment) -> Frame {
    let frame = tokio::time::timeout(Duration::from_secs(10), session.events.next())
        .await
        .expect("expected frame")
        .unwrap();
    if let Event::InteractionOpened { interaction } = &frame.event {
        let answer = if matches!(interaction.kind, InteractionKind::Permission { .. }) {
            Answer::AllowOnce
        } else {
            Answer::Confirm
        };
        session.handle.answer(
            IntentId::mint(),
            interaction.id.clone(),
            answer,
            Activation::Programmatic,
        );
    }
    frame
}
async fn open(host: &Host, root: &std::path::Path) -> Attachment {
    host.open(
        SessionSelector::Create {
            spec: SessionSpec {
                cwd: root.into(),
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
    .unwrap()
}
async fn turn(session: &mut Attachment, text: &str, complete: bool) {
    session
        .handle
        .submit(IntentId::mint(), Input::text(text, Origin::surface("test")));
    loop {
        let event = frame(session).await.event;
        if complete && matches!(event, Event::TurnCompleted { .. })
            || !complete && matches!(event, Event::TurnStarted { .. })
        {
            break;
        }
    }
}
async fn invoke_row(host: &Host, session: &mut Attachment, name: &str, input: Value) -> Value {
    host.extend(
        &session.session,
        "driver",
        "dollar-fixture",
        json!({"name":name,"input":input}),
    )
    .await
    .unwrap();
    loop {
        if let Event::Extension { kind, payload, .. } = frame(session).await.event
            && kind == "dollar-answer"
        {
            assert_eq!(payload["name"], name);
            assert!(payload.get("error").is_none(), "{name}: {payload}");
            return payload["value"].clone();
        }
    }
}
async fn counted(counts: &Counts, name: &str, expected: usize) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while counts.get(name) < expected {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("{name} was not raised"));
    assert_eq!(counts.get(name), expected, "{name} was raised twice");
}
#[tokio::test]
async fn every_row_raises_exactly_one_chain_on_the_real_host() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("fixture");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("plugin.json"),json!({"name":"fixture","version":"1","module":"main.ts","uses":ROWS.iter().map(|row|row.noun).collect::<Vec<_>>()} ).to_string()).unwrap();
    std::fs::write(
        root.join("main.ts"),
        include_str!("../../tests/mods/nouns/all.ts"),
    )
    .unwrap();
    let counts = Arc::new(Counts::default());
    let host = Host::build(
        vec![
            Box::new(ModsPlugin::default().with_dirs(vec![root])),
            Box::new(FixturePlugin(counts.clone())),
        ],
        HostConfig::new(Env::rooted(home.path()))
            .with_layer("offline", json!({"provider":"offline","model":"offline"})),
    )
    .await
    .unwrap();
    let mut session = open(&host, home.path()).await;
    let mut held = open(&host, home.path()).await;
    turn(&mut session, "seed", true).await;
    turn(&mut held, "hold-model", false).await;
    for row in ROWS.iter().filter(|row| row.method != "abort") {
        let name = format!("{}.{}", row.noun, row.method);
        let target = if name == "tool.call" {
            &mut held
        } else {
            &mut session
        };
        let input = sample(&name, &target.session);
        let before = counts.get(&name);
        let received = counts.get("session.receive");
        let answer = invoke_row(&host, target, &name, input).await;
        assert!((row.accepts)(&answer), "{name}: {answer}");
        counted(&counts, &name, before + 1).await;
        if name == "session.send" {
            counted(&counts, "session.receive", received + 1).await;
        }
    }
    let before = counts.get("turn.abort");
    let input = sample("turn.abort", &held.session);
    invoke_row(&host, &mut held, "turn.abort", input).await;
    counted(&counts, "turn.abort", before + 1).await;
    host.shutdown().await;
}
