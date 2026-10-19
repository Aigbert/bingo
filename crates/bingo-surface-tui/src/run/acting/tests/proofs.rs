//! Compare the scene's kernel answers with a real Host and fake provider.
use super::*;
use bingo_sdk::{Attachment, Delivery, HostApi};
use futures::StreamExt;

#[path = "streaming.rs"]
mod streaming;

struct Kernel {
    host: Arc<bingo_core::Host>,
    attached: Attachment,
    _home: tempfile::TempDir,
}
impl Kernel {
    async fn new(responses: Value) -> Self {
        let script = serde_json::from_value(json!({"responses":responses})).unwrap();
        Self::with_provider(Box::new(bingo_provider_fake::FakePlugin::new(Arc::new(
            bingo_provider_fake::FakeProvider::new(script),
        ))))
        .await
    }
    async fn with_provider(provider: Box<dyn bingo_sdk::Plugin>) -> Self {
        let home = tempfile::tempdir().unwrap();
        let host = bingo_core::Host::build(
            vec![provider, Box::new(bingo_tool_bash::BashPlugin::default())],
            bingo_core::HostConfig::new(bingo_sdk::Env::rooted(home.path())),
        )
        .await
        .unwrap();
        let attached = host
            .open(
                SessionSelector::Create {
                    spec: bingo_sdk::SessionSpec {
                        cwd: home.path().into(),
                        ..Default::default()
                    },
                },
                identity(),
                Default::default(),
            )
            .await
            .unwrap();
        Self {
            host,
            attached,
            _home: home,
        }
    }
    async fn until(&mut self, stop: impl Fn(&Event) -> bool) -> Vec<Event> {
        tokio::time::timeout(Duration::from_secs(10), async {
            let mut events = Vec::new();
            while let Some(frame) = self.attached.events.next().await {
                self.attached.snapshot.apply(&frame);
                let done = stop(&frame.event);
                events.push(frame.event);
                if done {
                    return events;
                }
            }
            panic!("kernel stream ended");
        })
        .await
        .unwrap()
    }
    async fn running() -> Self {
        let mut kernel = Self::new(json!([{"steps":[{"delay":{"ms":60000}}]}])).await;
        kernel
            .attached
            .handle
            .submit(IntentId::mint(), Input::text("first", crate::origin()));
        kernel
            .until(|e| {
                matches!(
                    e,
                    Event::IntentAck {
                        outcome: IntentOutcome::TurnStarted { .. },
                        ..
                    }
                )
            })
            .await;
        kernel
    }
    async fn queue(&mut self) -> (IntentId, Input, Vec<Event>, Vec<Event>) {
        let input = Input::Text {
            text: "queued".into(),
            images: vec![],
            origin: crate::origin(),
            delivery: Delivery::Hold,
            headline: None,
        };
        let intent = IntentId::mint();
        let expected = answers::queued(&self.attached.snapshot, intent.clone(), &input, 1);
        self.attached.handle.submit(intent.clone(), input.clone());
        let actual = self
            .until(|e| matches!(e, Event::IntentAck { intent: seen, .. } if seen == &intent))
            .await;
        (intent, input, expected, actual)
    }
}

fn normalized(events: &[Event]) -> Value {
    let mut value = serde_json::to_value(events).unwrap();
    scrub(&mut value);
    value
}
fn scrub(value: &mut Value) {
    match value {
        Value::Array(values) => {
            for value in values {
                scrub(value);
            }
        }
        Value::Object(fields) => {
            for (key, value) in fields {
                if matches!(key.as_str(), "id" | "turn" | "intent") && value.is_string() {
                    *value = json!("id");
                } else if key == "inputs" {
                    if let Some(ids) = value.as_array_mut() {
                        for id in ids {
                            *id = json!("id");
                        }
                    }
                } else if key == "durationMs" && value.is_number() {
                    *value = json!(0);
                } else if key.ends_with("At") && value.is_string() {
                    *value = json!("stamp");
                } else {
                    scrub(value);
                }
            }
        }
        _ => {}
    }
}

#[tokio::test]
async fn queued_answer_matches_the_kernel() {
    let mut kernel = Kernel::running().await;
    let (_, _, expected, actual) = kernel.queue().await;
    assert_eq!(normalized(&actual), normalized(&expected));
    kernel.host.shutdown().await;
}

#[tokio::test]
async fn withdrawal_answer_matches_the_kernel() {
    let mut kernel = Kernel::running().await;
    let (intent, input, _, _) = kernel.queue().await;
    let expected = answers::removed(&kernel.attached.snapshot, &intent, 2);
    let answer = kernel
        .host
        .withdraw(&kernel.attached.session, &intent, identity())
        .await;
    assert_eq!(answer, answers::withdrawn(&input));
    let actual = kernel
        .until(|e| matches!(e, Event::QueueChanged { .. }))
        .await;
    assert_eq!(normalized(&actual), normalized(&[expected]));
    kernel.host.shutdown().await;
}

#[tokio::test]
async fn sent_answer_matches_the_kernel() {
    let mut kernel = Kernel::new(json!([{"steps":[{"delay":{"ms":60000}}]}])).await;
    let input = Input::text("sent", crate::origin());
    let intent = IntentId::mint();
    let expected = answers::sent("turn", intent.clone(), &input, jiff::Timestamp::UNIX_EPOCH);
    kernel.attached.handle.submit(intent, input);
    let actual = kernel
        .until(|e| {
            matches!(
                e,
                Event::IntentAck {
                    outcome: IntentOutcome::TurnStarted { .. },
                    ..
                }
            )
        })
        .await;
    let actual: Vec<_> = actual
        .into_iter()
        .filter(|e| {
            matches!(
                e,
                Event::ItemCompleted { .. } | Event::TurnStarted { .. } | Event::IntentAck { .. }
            )
        })
        .collect();
    assert_eq!(normalized(&actual), normalized(&expected));
    kernel.host.shutdown().await;
}

#[tokio::test]
async fn stop_answer_matches_the_kernel() {
    let provider = streaming::Streaming(Arc::new(bingo_provider_fake::FakeProvider::demo()));
    let mut kernel = Kernel::with_provider(Box::new(provider)).await;
    kernel
        .attached
        .handle
        .submit(IntentId::mint(), Input::text("start", crate::origin()));
    kernel
        .until(|e| matches!(e, Event::ItemDelta { data, .. } if data.ends_with('.')))
        .await;
    let expected = answers::stopped(
        &kernel.attached.snapshot,
        jiff::Timestamp::UNIX_EPOCH,
        answers::StopPhase::Streaming,
    );
    kernel
        .attached
        .handle
        .interrupt(IntentId::mint(), InterruptScope::Head);
    let actual = kernel
        .until(|e| matches!(e, Event::TurnCompleted { .. }))
        .await;
    let actual: Vec<_> = actual
        .into_iter()
        .filter(|e| matches!(e, Event::ItemCompleted { .. } | Event::TurnCompleted { .. }))
        .collect();
    assert!(actual.iter().any(|e| matches!(e, Event::ItemCompleted { item } if item.status == bingo_sdk::ItemStatus::Interrupted && matches!(item.body, ItemBody::Assistant { .. }))));
    assert_eq!(normalized(&actual), normalized(&expected));
    kernel.host.shutdown().await;
}

#[tokio::test]
async fn stop_answer_matches_the_kernel_for_a_running_tool() {
    // sleep is available in both Git Bash on Windows and Unix shells.
    let mut kernel = Kernel::new(
        json!([{"steps":[{"toolCall":{"name":"Bash","input":{"command":"sleep 60 && echo done"}}}]}]),
    )
    .await;
    kernel
        .attached
        .handle
        .submit(IntentId::mint(), Input::text("start", crate::origin()));
    kernel.until(|e| matches!(e, Event::ItemStarted { item } if matches!(item.body, ItemBody::ToolCall { .. }))).await;
    kernel
        .until(|e| matches!(e, Event::InteractionOpened { .. }))
        .await;
    let ask = kernel.attached.snapshot.interactions[0].id.clone();
    kernel.attached.handle.answer(
        IntentId::mint(),
        ask,
        bingo_sdk::Answer::AllowOnce,
        bingo_sdk::Activation::Pointer,
    );
    kernel.until(|e| matches!(e, Event::ItemUpdated { item } if item.status == bingo_sdk::ItemStatus::Running && matches!(item.body, ItemBody::ToolCall { .. }))).await;
    let expected = answers::stopped(
        &kernel.attached.snapshot,
        jiff::Timestamp::UNIX_EPOCH,
        answers::StopPhase::Tools,
    );
    kernel
        .attached
        .handle
        .interrupt(IntentId::mint(), InterruptScope::Head);
    let actual = kernel
        .until(|e| matches!(e, Event::TurnCompleted { .. }))
        .await;
    let actual: Vec<_> = actual
        .into_iter()
        .filter(|e| matches!(e, Event::ItemCompleted { .. }))
        .collect();
    let expected: Vec<_> = expected
        .into_iter()
        .filter(|e| matches!(e, Event::ItemCompleted { .. }))
        .collect();
    assert_eq!(normalized(&actual), normalized(&expected));
    kernel.host.shutdown().await;
}

#[tokio::test]
async fn stop_answer_matches_the_kernel_for_a_queued_chain_call() {
    // sleep is available in both Git Bash on Windows and Unix shells.
    let mut kernel = Kernel::new(json!([{"steps":[
        {"toolCall":{"name":"Bash","input":{"command":"sleep 60 && echo done"}}},
        {"toolCall":{"name":"Bash","input":{"command":"echo queued"}}}
    ]}]))
    .await;
    kernel
        .attached
        .handle
        .submit(IntentId::mint(), Input::text("start", crate::origin()));
    kernel.until(|e| matches!(e, Event::ItemStarted { item } if matches!(item.body, ItemBody::ToolCall { .. }))).await;
    kernel
        .until(|e| matches!(e, Event::InteractionOpened { .. }))
        .await;
    let ask = kernel.attached.snapshot.interactions[0].id.clone();
    kernel.attached.handle.answer(
        IntentId::mint(),
        ask,
        bingo_sdk::Answer::AllowOnce,
        bingo_sdk::Activation::Pointer,
    );
    kernel.until(|e| matches!(e, Event::ItemUpdated { item } if item.status == bingo_sdk::ItemStatus::Running && matches!(item.body, ItemBody::ToolCall { .. }))).await;
    let queued = kernel.attached.snapshot.transcript.items().iter().find(|item| {
        matches!(&item.body, ItemBody::ToolCall { input, .. } if input["command"] == "echo queued")
    }).unwrap();
    assert_eq!(queued.status, bingo_sdk::ItemStatus::Pending);
    let queued_id = queued.id.clone();
    let expected = answers::stopped(
        &kernel.attached.snapshot,
        jiff::Timestamp::UNIX_EPOCH,
        answers::StopPhase::Tools,
    );
    kernel
        .attached
        .handle
        .interrupt(IntentId::mint(), InterruptScope::Head);
    let actual = kernel
        .until(|e| matches!(e, Event::TurnCompleted { .. }))
        .await;
    let actual: Vec<_> = actual
        .into_iter()
        .filter(|e| matches!(e, Event::ItemCompleted { .. }))
        .collect();
    let expected: Vec<_> = expected
        .into_iter()
        .filter(|e| matches!(e, Event::ItemCompleted { .. }))
        .collect();
    assert!(actual.iter().any(|e| matches!(e, Event::ItemCompleted { item }
        if item.id == queued_id && item.status == bingo_sdk::ItemStatus::Interrupted
            && matches!(item.body, ItemBody::ToolCall { duration_ms: Some(0), began_at: None, .. }))));
    assert_eq!(normalized(&actual), normalized(&expected));
    kernel.host.shutdown().await;
}

#[tokio::test]
async fn stop_answer_leaves_streamed_tool_input_without_a_completion() {
    let mut kernel = Kernel::new(json!([{"steps":[
        {"toolCall":{"name":"Bash","input":{"command":"echo pending"}}},
        {"delay":{"ms":60000}}
    ]}]))
    .await;
    kernel
        .attached
        .handle
        .submit(IntentId::mint(), Input::text("start", crate::origin()));
    kernel.until(|e| matches!(e, Event::ItemStarted { item } if matches!(item.body, ItemBody::ToolCall { .. }))).await;
    let expected = answers::stopped(
        &kernel.attached.snapshot,
        jiff::Timestamp::UNIX_EPOCH,
        answers::StopPhase::Streaming,
    );
    kernel
        .attached
        .handle
        .interrupt(IntentId::mint(), InterruptScope::Head);
    let actual = kernel
        .until(|e| matches!(e, Event::TurnCompleted { .. }))
        .await;
    let actual: Vec<_> = actual
        .into_iter()
        .filter(|e| matches!(e, Event::ItemCompleted { .. }))
        .collect();
    let expected: Vec<_> = expected
        .into_iter()
        .filter(|e| matches!(e, Event::ItemCompleted { .. }))
        .collect();
    assert!(!actual.iter().any(|e| matches!(e, Event::ItemCompleted { item } if matches!(item.body, ItemBody::ToolCall { .. }))));
    assert_eq!(normalized(&actual), normalized(&expected));
    kernel.host.shutdown().await;
}

#[tokio::test]
async fn shell_answer_matches_the_kernel() {
    let mut kernel = Kernel::new(json!([])).await;
    // echo is available in Git Bash on Windows and in the Unix shell.
    kernel.attached.handle.submit(
        IntentId::mint(),
        Input::text("!echo scene", crate::origin()),
    );
    let actual = kernel.until(|e| matches!(e, Event::ItemCompleted { item } if matches!(item.body, ItemBody::Shell { .. }))).await;
    let actual: Vec<_> = actual
        .into_iter()
        .filter(|e| matches!(e, Event::ItemCompleted { .. }))
        .collect();
    let expected = answers::shell(
        "!echo scene",
        "scene\n",
        &kernel.attached.snapshot.summary.cwd,
        jiff::Timestamp::UNIX_EPOCH,
        "shell",
        IntentId::from_raw("intent"),
    );
    assert_eq!(normalized(&actual), normalized(&[expected]));
    kernel.host.shutdown().await;
}

#[tokio::test]
async fn steer_answer_matches_the_kernel() {
    let mut kernel = Kernel::new(json!([{"steps":[{"toolCall":{"name":"Bash","input":{"command":"echo scene"}}}]}, {"steps":[{"text":"done"}]}])).await;
    kernel
        .attached
        .handle
        .submit(IntentId::mint(), Input::text("start", crate::origin()));
    kernel
        .until(|e| matches!(e, Event::InteractionOpened { .. }))
        .await;
    let ask = kernel.attached.snapshot.interactions[0].id.clone();
    let input = Input::text("steered", crate::origin());
    let intent = IntentId::mint();
    kernel.attached.handle.submit(intent.clone(), input.clone());
    kernel
        .until(|e| matches!(e, Event::IntentAck { intent: seen, .. } if seen == &intent))
        .await;
    let expected = answers::steered(
        &kernel.attached.snapshot,
        &intent,
        &input,
        "steer",
        jiff::Timestamp::UNIX_EPOCH,
        2,
    );
    kernel.attached.handle.answer(
        IntentId::mint(),
        ask,
        bingo_sdk::Answer::AllowOnce,
        bingo_sdk::Activation::Pointer,
    );
    let actual = kernel.until(|e| matches!(e, Event::ItemCompleted { item } if matches!(&item.body, ItemBody::User { parts, .. } if parts.iter().any(|p| p.as_text() == Some("steered"))))).await;
    let actual: Vec<_> = actual.into_iter().filter(|e| matches!(e, Event::QueueChanged { .. }) || matches!(e, Event::ItemCompleted { item } if matches!(item.body, ItemBody::User { .. }))).collect();
    assert_eq!(normalized(&actual), normalized(&expected));
    kernel.host.shutdown().await;
}
