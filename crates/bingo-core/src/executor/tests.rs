use super::*;
use crate::test_support::{EchoTool, NoHost};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Mutex;
use tokio::sync::{mpsc, oneshot};

struct Released {
    safe: bool,
    started: mpsc::UnboundedSender<()>,
    release: Mutex<Option<oneshot::Receiver<()>>>,
}

#[async_trait]
impl Tool for Released {
    fn spec(&self) -> ToolSpec {
        EchoTool { read_only: true }.spec()
    }
    fn traits(&self, _: &Value) -> ToolTraits {
        ToolTraits {
            concurrency_safe: self.safe,
            ..ToolTraits::default()
        }
    }
    async fn call(&self, _: Value, _: &ToolContext) -> Result<ToolOutput, ToolError> {
        let release = self.release.lock().unwrap().take().unwrap();
        self.started.send(()).unwrap();
        release.await.unwrap();
        Ok(ToolOutput::text("landed"))
    }
}

fn pending(
    id: &str,
    safe: bool,
) -> (
    PendingCall,
    oneshot::Sender<()>,
    mpsc::UnboundedReceiver<()>,
) {
    let (release, receive) = oneshot::channel();
    let (started, starts) = mpsc::unbounded_channel();
    let tool = Arc::new(Released {
        safe,
        started,
        release: Mutex::new(Some(receive)),
    });
    let traits = tool.traits(&json!({}));
    (
        PendingCall {
            item: ItemId::from_raw(id),
            call: ToolCall {
                call_id: id.into(),
                name: "Echo".into(),
                input: json!({}),
            },
            tool: Some(tool),
            traits,
            gate: Gate::Allowed,
        },
        release,
        starts,
    )
}

fn context(cancel: &CancellationToken) -> ToolContext {
    ToolContext {
        call_id: "c".into(),
        session: SessionId::from_raw("s"),
        turn: TurnId::from_raw("t"),
        item: ItemId::from_raw("i"),
        cwd: "/tmp".into(),
        cancel: cancel.child_token(),
        env: Arc::new(Env {
            home: "/tmp".into(),
            config_dir: "/tmp".into(),
            data_dir: "/tmp".into(),
        }),
        host: bingo_sdk::testing::NoHost::handle(),
        call: Arc::new(NoHost),
    }
}

#[tokio::test]
async fn start_polls_the_call_and_each_landing_frees_its_own_slot() {
    let cancel = CancellationToken::new();
    let mut flight = InFlight::default();
    let (first, a, mut a_started) = pending("first", true);
    let (second, b, mut b_started) = pending("second", true);
    assert!(
        flight
            .start(first, context(&cancel), cancel.clone(), None)
            .is_none()
    );
    assert!(a_started.try_recv().is_ok(), "start means started");
    assert!(
        flight
            .start(second, context(&cancel), cancel.clone(), None)
            .is_none()
    );
    assert!(b_started.try_recv().is_ok());
    assert_eq!(flight.state().count, 2);
    assert!(flight.state().all_safe);
    b.send(()).unwrap();
    assert_eq!(flight.next().await.unwrap().call_id, "second");
    assert_eq!(flight.state().count, 1);
    a.send(()).unwrap();
    assert_eq!(flight.next().await.unwrap().call_id, "first");
    assert_eq!(flight.state().count, 0);
    assert!(flight.next().await.is_none());
}

#[tokio::test]
async fn an_exclusive_call_is_visible_in_the_flight_state() {
    let cancel = CancellationToken::new();
    let mut flight = InFlight::default();
    let (call, release, _started) = pending("exclusive", false);
    assert!(
        flight
            .start(call, context(&cancel), cancel.clone(), None)
            .is_none()
    );
    assert!(!flight.state().all_safe);
    release.send(()).unwrap();
    assert_eq!(flight.next().await.unwrap().status, ItemStatus::Completed);
    assert!(flight.state().all_safe);
}

#[tokio::test]
async fn an_interrupt_before_start_never_polls_the_tool() {
    let cancel = CancellationToken::new();
    cancel.cancel();
    let mut flight = InFlight::default();
    let (call, _, mut started) = pending("never", true);
    let out = flight
        .start(call, context(&cancel), cancel.clone(), None)
        .unwrap();
    assert_eq!(out.status, ItemStatus::Interrupted);
    assert!(started.try_recv().is_err());
}

#[tokio::test]
async fn a_tool_that_ignores_its_token_still_ends_on_the_interrupt() {
    let cancel = CancellationToken::new();
    let mut flight = InFlight::default();
    let (first, a, _started_a) = pending("first", true);
    let (second, b, _started_b) = pending("second", true);
    flight.start(first, context(&cancel), cancel.clone(), None);
    flight.start(second, context(&cancel), cancel.clone(), None);
    cancel.cancel();
    let outcomes: Vec<_> = flight.cancelled().collect();
    assert_eq!(outcomes.len(), 2);
    assert!(outcomes.iter().all(|o| o.status == ItemStatus::Interrupted));
    assert!(
        outcomes
            .iter()
            .all(|o| o.output.parts[0].as_text() == Some(INTERRUPTED_MARKER))
    );
    assert!(
        a.is_closed() && b.is_closed(),
        "both tool futures were dropped"
    );
}

#[tokio::test]
async fn denied_and_missing_tools_fail_without_entering_flight() {
    let cancel = CancellationToken::new();
    let mut flight = InFlight::default();
    let (mut denied, _release, mut started) = pending("denied", true);
    denied.gate = Gate::Denied {
        message: "no".into(),
    };
    let out = flight
        .start(denied, context(&cancel), cancel.clone(), None)
        .unwrap();
    assert_eq!(out.status, ItemStatus::Failed);
    assert_eq!(out.output.parts[0].as_text(), Some("no"));
    assert!(started.try_recv().is_err());
    let (mut missing, _, _) = pending("missing", true);
    missing.tool = None;
    let out = flight
        .start(missing, context(&cancel), cancel.clone(), None)
        .unwrap();
    assert_eq!(out.status, ItemStatus::Failed);
    assert_eq!(out.output.parts[0].as_text(), Some("tool not found: Echo"));
    assert_eq!(flight.state().count, 0);
}

#[test]
fn results_are_clipped_at_the_global_cap() {
    let mut out = ToolOutput::text("x".repeat(MAX_RESULT_CHARS + 5));
    clip(&mut out);
    assert!(
        out.parts[0]
            .as_text()
            .unwrap()
            .ends_with("[truncated: 5 more characters]")
    );
}
