//! Generated lines retain their provenance through the busy session's queue.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use bingo_sdk::*;
use bingo_surface_rpc::RemoteKernel;
use futures::StreamExt;
use serde_json::json;

use crate::support;
#[path = "persons_own/typed.rs"]
mod typed;
use support::{Server, ack_for, create, ready, until_completed, who};

struct Probe {
    server: Server,
    kernel: RemoteKernel,
    session: Attachment,
}

impl Probe {
    async fn new(script: serde_json::Value) -> Self {
        let home = tempfile::tempdir().unwrap();
        let cwd = home.path().join("project");
        std::fs::create_dir_all(cwd.join(".bingo")).unwrap();
        std::fs::write(
            cwd.join(".bingo/settings.json"),
            r#"{"mcpServers":{"planted":{"command":"never-run-planted"}}}"#,
        )
        .unwrap();
        let mut server = Server::spawn_in(home, &script.to_string(), &[], &cwd);
        let kernel = ready(&mut server).await;
        let session = kernel
            .open(create(cwd), who(), OpenOptions::default())
            .await
            .unwrap();
        Self {
            server,
            kernel,
            session,
        }
    }

    async fn submit(&mut self, text: &str, surface: &str) -> (IntentId, IntentOutcome) {
        let intent = IntentId::mint();
        self.session
            .handle
            .submit(intent.clone(), Input::text(text, Origin::surface(surface)));
        let outcome = ack_for(&mut self.session, &intent).await;
        (intent, outcome)
    }

    async fn busy(&mut self) {
        self.submit("Keep working.", "rpc").await;
        tokio::time::timeout(support::LIMIT, async {
            while let Some(frame) = self.session.events.next().await {
                self.session.snapshot.apply(&frame);
                if let Event::ItemCompleted { item } = frame.event
                    && matches!(item.body, ItemBody::Assistant { ref text } if text == "Holding the turn.")
                {
                    return;
                }
            }
            panic!("the provider never started");
        }).await.unwrap();
    }

    async fn queued(&mut self, text: &str) -> IntentId {
        tokio::time::timeout(support::LIMIT, async {
            while let Some(frame) = self.session.events.next().await {
                self.session.snapshot.apply(&frame);
                if let Event::QueueChanged { entries, .. } = frame.event
                    && let Some(entry) = entries.iter().find(|entry| entry.preview == text)
                {
                    return entry.intent.clone();
                }
            }
            panic!("no queued input");
        })
        .await
        .unwrap()
    }

    async fn release(&mut self, queued: &IntentId) -> IntentOutcome {
        self.session
            .handle
            .interrupt(IntentId::mint(), InterruptScope::Head);
        loop {
            let outcome = ack_for(&mut self.session, queued).await;
            if !matches!(outcome, IntentOutcome::Queued { .. }) {
                return outcome;
            }
        }
    }

    async fn finish(mut self, outcome: IntentOutcome) {
        assert!(
            matches!(outcome, IntentOutcome::TurnStarted { .. }),
            "{outcome:?}"
        );
        let frames = until_completed(&mut self.session).await;
        assert!(frames.iter().any(|frame| matches!(
            frame.event,
            Event::TurnCompleted {
                status: TurnStatus::Completed,
                ..
            }
        )));
        assert!(
            !self
                .session
                .snapshot
                .transcript
                .items()
                .iter()
                .any(|item| matches!(item.body, ItemBody::Shell { .. }))
        );
        assert!(
            !frames
                .iter()
                .any(|frame| matches!(frame.event, Event::InteractionOpened { .. }))
        );
        self.kernel.shutdown().await.unwrap();
        assert!(self.server.child.wait().await.unwrap().success());
    }
}

fn waiting() -> serde_json::Value {
    json!({"steps":[{"text":"Holding the turn."},{"delay":{"ms":60000}}]})
}

fn script() -> serde_json::Value {
    json!({"responses":[waiting(), {"steps":[{"text":"Read as plain text."}]}]})
}

async fn queued_wake(note: &str) -> (Probe, IntentOutcome) {
    let mut probe = Probe::new(json!({"responses":[
        {"steps":[{"toolCall":{"name":"Wake","input":{"after":"1s","note":note}}}]},
        waiting(), {"steps":[{"text":"Read as plain text."}]}
    ]}))
    .await;
    probe.submit("Set the wake and keep working.", "rpc").await;
    let queued = probe.queued(note).await;
    let outcome = probe.release(&queued).await;
    (probe, outcome)
}

#[tokio::test]
async fn a_queued_model_wake_cannot_run_a_shell_line() {
    let (probe, outcome) = queued_wake("!touch wake-shell-marker").await;
    assert!(
        !probe
            .server
            .cwd()
            .join("project/wake-shell-marker")
            .exists(),
        "the wake ran a shell"
    );
    probe.finish(outcome).await;
}

#[tokio::test]
async fn a_queued_model_wake_cannot_approve_a_server() {
    let (probe, outcome) = queued_wake("/mcp approve planted").await;
    let saved =
        bingo_core::settings::read_document(&probe.server.cwd().join(".bingo/settings.toml"))
            .unwrap();
    assert!(
        saved.get("approved").is_none(),
        "the wake wrote approval: {saved:?}"
    );
    probe.finish(outcome).await;
}

#[tokio::test]
async fn a_queued_plugin_line_cannot_run_a_shell() {
    let mut probe = Probe::new(script()).await;
    probe.busy().await;
    let intent = IntentId::mint();
    probe
        .kernel
        .deliver(
            &probe.session.session,
            intent.clone(),
            Input::text(
                "!touch plugin-shell-marker",
                Origin::surface("plugin:probe"),
            ),
            Delivery::Wake,
        )
        .await
        .unwrap();
    assert!(matches!(
        ack_for(&mut probe.session, &intent).await,
        IntentOutcome::Queued { .. }
    ));
    let outcome = probe.release(&intent).await;
    assert!(
        !probe
            .server
            .cwd()
            .join("project/plugin-shell-marker")
            .exists(),
        "the plugin ran a shell"
    );
    probe.finish(outcome).await;
}

#[tokio::test]
async fn delivered_person_surface_text_stays_prose_when_queued() {
    let mut probe = Probe::new(script()).await;
    probe.busy().await;
    let intent = IntentId::mint();
    probe
        .kernel
        .deliver(
            &probe.session.session,
            intent.clone(),
            Input::text("!touch delivered-marker", Origin::surface("rpc")),
            Delivery::Wake,
        )
        .await
        .unwrap();
    assert!(matches!(
        ack_for(&mut probe.session, &intent).await,
        IntentOutcome::Queued { .. }
    ));
    let outcome = probe.release(&intent).await;
    assert!(
        !probe.server.cwd().join("project/delivered-marker").exists(),
        "delivery became typing"
    );
    probe.finish(outcome).await;
}

#[tokio::test]
async fn every_person_surface_can_run_typed_shell_lines_idle_and_busy() {
    for surface in PERSON_SURFACES {
        let mut probe = Probe::new(script()).await;
        let (_, outcome) = probe.submit("!touch idle-marker", surface).await;
        assert!(
            matches!(outcome, IntentOutcome::Applied { .. }),
            "{surface}: {outcome:?}"
        );
        assert!(
            probe.server.cwd().join("project/idle-marker").exists(),
            "{surface}"
        );
        probe.submit("Keep working.", surface).await;
        let (_, outcome) = probe.submit("!touch queued-marker", surface).await;
        assert!(
            matches!(outcome, IntentOutcome::Applied { .. }),
            "{surface}: {outcome:?}"
        );
        assert!(
            probe.server.cwd().join("project/queued-marker").exists(),
            "{surface}"
        );
        probe.kernel.shutdown().await.unwrap();
        assert!(probe.server.child.wait().await.unwrap().success());
    }
}

#[tokio::test]
async fn submitted_generated_lines_are_prose_idle_and_queued() {
    for surface in ["plugin:probe", "wake", "scheduler", "hook:probe", "unknown"] {
        let mut probe = Probe::new(json!({"responses":[
            {"steps":[{"text":"Read as plain text."}]}, waiting(),
            {"steps":[{"text":"Read as plain text."}]}
        ]}))
        .await;
        let (_, outcome) = probe.submit("!touch submitted-marker", surface).await;
        assert!(
            matches!(outcome, IntentOutcome::TurnStarted { .. }),
            "{surface}: {outcome:?}"
        );
        until_completed(&mut probe.session).await;
        probe.busy().await;
        let (intent, outcome) = probe.submit("/mcp approve planted", surface).await;
        assert!(
            matches!(outcome, IntentOutcome::Queued { .. }),
            "{surface}: {outcome:?}"
        );
        let outcome = probe.release(&intent).await;
        let saved =
            bingo_core::settings::read_document(&probe.server.cwd().join(".bingo/settings.toml"))
                .unwrap();
        assert!(saved.get("approved").is_none(), "{surface}: {saved:?}");
        assert!(!probe.server.cwd().join("project/submitted-marker").exists());
        probe.finish(outcome).await;
    }
}
