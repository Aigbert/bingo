//! Ownership contracts: a hung plugin cannot hold the session after esc.

use super::*;
use crate::test_support::owned::Hung;
use crate::turn::{HookSet, TurnConfig};
use std::time::Duration;

const GRACE: Duration = Duration::from_secs(2);

async fn bounded<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(GRACE + Duration::from_millis(1), future)
        .await
        .expect("the actor must respond by the interrupt grace")
}

fn configured(cfg: TurnConfig, commands: Vec<Arc<dyn Command>>) -> Mailbox {
    spawn(summary("ses_owned"), None, services(commands), |_| {
        Arc::new(cfg)
    })
}

fn input(text: &str) -> Input {
    Input::text(text, Origin::surface("test"))
}

fn tool_config() -> TurnConfig {
    config(
        ScriptedProvider::new(vec![Script::Events(tool_call("echo", json!({"v": 1})))]),
        vec![Arc::new(EchoTool { read_only: true })],
        Arc::new(NoHost),
    )
}

async fn stopped_turn(cfg: TurnConfig, hung: Arc<Hung>, text: &str) {
    let mailbox = configured(cfg, vec![]);
    let (mut state, mut events) = mailbox.attach().await.unwrap();
    mailbox.submit(IntentId::mint(), input(text));
    bounded(hung.entered.notified()).await;
    mailbox.interrupt(IntentId::mint(), InterruptScope::Head);
    // The round trip proves the actor has processed the interrupt before time advances.
    bounded(mailbox.summary()).await.unwrap();
    tokio::time::advance(GRACE + Duration::from_millis(1)).await;
    bounded(drive(&mut events, &mut state, turn_completed)).await;
    assert_eq!(
        state.last_status(),
        Some(&TurnStatus::Interrupted {
            reason: InterruptReason::UserCancel
        })
    );
    assert!(
        state
            .transcript
            .items()
            .iter()
            .all(|item| !matches!(item.status, ItemStatus::Pending | ItemStatus::Running))
    );
    assert!(
        state
            .transcript
            .items()
            .iter()
            .any(|item| matches!(item.body, ItemBody::Interruption { .. }))
    );
    mailbox.close(CloseReason::Client);
    bounded(mailbox.wait_closed()).await;
}

#[tokio::test(start_paused = true)]
async fn interrupt_drops_a_hung_before_tool_hook() {
    let hung = Hung::at(HookPoint::BeforeTool);
    let mut cfg = tool_config();
    cfg.hooks = HookSet::fixed(vec![hung.clone()]);
    stopped_turn(cfg, hung, "go").await;
}

#[tokio::test(start_paused = true)]
async fn interrupt_drops_a_hung_policy() {
    let hung = Hung::at(HookPoint::BeforeTool);
    let mut cfg = tool_config();
    cfg.policy = hung.clone();
    stopped_turn(cfg, hung, "go").await;
}

#[tokio::test(start_paused = true)]
async fn interrupt_drops_a_compactor_that_ignores_its_token() {
    let hung = Hung::at(HookPoint::Compact);
    let mut cfg = tool_config();
    cfg.model.as_mut().unwrap().capabilities.context_window = 64;
    cfg.model.as_mut().unwrap().max_tokens = 8;
    cfg.compactor.fixed = Some(hung.clone());
    stopped_turn(cfg, hung, &"large input ".repeat(100)).await;
}

async fn rejected(events: &mut FrameStream, state: &mut SessionState, intent: &IntentId) {
    let frames = bounded(frames_until(events, state, |frame| {
        matches!(&frame.event, Event::IntentAck { intent: id, outcome: IntentOutcome::Rejected { .. } } if id == intent)
    })).await;
    assert!(matches!(&frames.last().unwrap().event,
        Event::IntentAck { outcome: IntentOutcome::Rejected { error }, .. } if error.code.as_str() == "INTERRUPTED"));
}

#[tokio::test(start_paused = true)]
async fn interrupt_releases_a_holding_command_and_its_queue() {
    let hung = Hung::at(HookPoint::Submit);
    let cfg = config(
        ScriptedProvider::new(vec![Script::Events(text("after"))]),
        vec![],
        Arc::new(NoHost),
    );
    let mailbox = configured(cfg, vec![hung.clone()]);
    let (mut state, mut events) = mailbox.attach().await.unwrap();
    let intent = IntentId::mint();
    mailbox.submit(intent.clone(), input("/hang"));
    bounded(hung.entered.notified()).await;
    mailbox.submit(IntentId::mint(), input("after"));
    mailbox.interrupt(IntentId::mint(), InterruptScope::Head);
    bounded(mailbox.summary()).await.unwrap();
    tokio::time::advance(GRACE + Duration::from_millis(1)).await;
    rejected(&mut events, &mut state, &intent).await;
    bounded(drive(&mut events, &mut state, turn_completed)).await;
    assert_eq!(user_texts(&state), ["after"]);
    mailbox.close(CloseReason::Client);
    bounded(mailbox.wait_closed()).await;
}

#[tokio::test(start_paused = true)]
async fn a_hung_submit_keeps_queries_and_other_sessions_responsive() {
    let hung = Hung::at(HookPoint::Submit);
    let mut cfg = config(
        ScriptedProvider::new(vec![Script::Events(text("after"))]),
        vec![],
        Arc::new(NoHost),
    );
    cfg.hooks = HookSet::fixed(vec![hung.clone()]);
    let mailbox = configured(cfg, vec![]);
    let (mut state, mut events) = mailbox.attach().await.unwrap();
    let intent = IntentId::mint();
    mailbox.submit(intent.clone(), input("hang"));
    bounded(hung.entered.notified()).await;
    let second = start(
        ScriptedProvider::new(vec![Script::Events(text("independent"))]),
        vec![],
    );
    let (mut other, mut stream) = second.attach().await.unwrap();
    second.submit(IntentId::mint(), input("go"));
    bounded(drive(&mut stream, &mut other, turn_completed)).await;
    bounded(mailbox.summary()).await.unwrap();
    mailbox.submit(IntentId::mint(), input("after"));
    mailbox.interrupt(IntentId::mint(), InterruptScope::Head);
    bounded(mailbox.summary()).await.unwrap();
    tokio::time::advance(GRACE + Duration::from_millis(1)).await;
    rejected(&mut events, &mut state, &intent).await;
    bounded(drive(&mut events, &mut state, turn_completed)).await;
    assert_eq!(user_texts(&state), ["after"]);
    mailbox.close(CloseReason::Client);
    second.close(CloseReason::Client);
    bounded(mailbox.wait_closed()).await;
    bounded(second.wait_closed()).await;
}
