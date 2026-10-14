//! The policy's own view of a session reaches the clients as
//! `ConfigView.plugins[policy.id()]` (ADR-0009 §5).

use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

static POLICY: PluginManifest = PluginManifest {
    id: "test.policy",
    version: "0",
    uses: &[],
    provides: &["policy:counting"],
    requires: &[],
    config: None,
};

/// Asks for everything and counts the verdicts it is told about.
struct CountingPolicy {
    verdicts: AtomicUsize,
}

#[async_trait]
impl PermissionPolicy for CountingPolicy {
    fn id(&self) -> &str {
        "counting"
    }
    async fn decide(&self, _: PolicyInput<'_>) -> Decision {
        Decision::Ask {
            reason: Reason::Default,
            scope: None,
        }
    }
    async fn on_verdict(&self, _: PolicyInput<'_>, _: &Verdict) {
        self.verdicts.fetch_add(1, Ordering::SeqCst);
    }
    fn describe(&self, _: &SessionId) -> serde_json::Value {
        json!({ "verdicts": self.verdicts.load(Ordering::SeqCst) })
    }
}

#[tokio::test]
async fn the_policys_view_is_published_at_open_and_after_a_verdict() {
    let receipts = Arc::new(std::sync::Mutex::new(Vec::<Item>::new()));
    let provider = ScriptedProvider::new(vec![
        Script::Events(tool_call("Echo", json!({ "v": 1 }))),
        Script::Events(text("done")),
    ]);
    let plugins = vec![
        TestPlugin::boxed(&PROVIDER, vec![Contribution::Provider(provider)]),
        TestPlugin::boxed(
            &TOOLS,
            vec![Contribution::Tool(Arc::new(EchoTool { read_only: true }))],
        ),
        TestPlugin::boxed(
            &POLICY,
            vec![
                Contribution::Policy(Arc::new(CountingPolicy {
                    verdicts: AtomicUsize::new(0),
                })),
                Contribution::Hook(Arc::new(CaptureReceipts(receipts.clone()))),
            ],
        ),
    ];
    let config = HostConfig::new(env()).with_layer("cli", json!({ "model": "m" }));
    let host = Host::build(plugins, config).await.unwrap();
    let mut attachment = host
        .open(
            SessionSelector::Create {
                spec: spec("/work"),
            },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(
        attachment.snapshot.config.plugins["counting"],
        json!({ "verdicts": 0 }),
        "the view is part of the snapshot from the start"
    );

    attachment.handle.submit(
        IntentId::mint(),
        Input::text("echo", Origin::surface("test")),
    );
    let mut order = Vec::new();
    while let Some(frame) = attachment.events.next().await {
        attachment.snapshot.apply(&frame);
        match &frame.event {
            // A refusal is a verdict the policy hears about; a plain allow
            // installs nothing and is not one.
            Event::InteractionOpened { interaction } => attachment.handle.answer(
                IntentId::mint(),
                interaction.id.clone(),
                Answer::Deny { feedback: None },
                Activation::Keyboard,
            ),
            Event::ItemCompleted { item }
                if matches!(item.body, ItemBody::PermissionReceipt { .. }) =>
            {
                order.push("receipt")
            }
            Event::ConfigChanged { .. } => order.push("config"),
            Event::TurnCompleted { .. } => break,
            _ => {}
        }
    }
    let published: Vec<_> = attachment
        .snapshot
        .transcript
        .items()
        .iter()
        .filter(|item| matches!(item.body, ItemBody::PermissionReceipt { .. }))
        .map(|item| item.as_ref().clone())
        .collect();
    assert_eq!(
        *receipts.lock().unwrap(),
        published,
        "the turn and stream must hold the same receipt"
    );
    assert_eq!(
        order,
        vec!["receipt", "config"],
        "the verdict is announced right after its receipt"
    );
    assert_eq!(
        attachment.snapshot.config.plugins["counting"],
        json!({ "verdicts": 1 })
    );
}

struct CaptureReceipts(Arc<std::sync::Mutex<Vec<Item>>>);
#[async_trait]
impl Hook for CaptureReceipts {
    fn id(&self) -> &str {
        "capture.receipts"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![HookPoint::Turn],
            tool: None,
        }
    }
    async fn on_turn(&self, phase: Phase, _: &TurnId, items: &[Item], _: &HookContext) {
        if phase == Phase::End {
            *self.0.lock().unwrap() = items
                .iter()
                .filter(|item| matches!(item.body, ItemBody::PermissionReceipt { .. }))
                .cloned()
                .collect();
        }
    }
}
