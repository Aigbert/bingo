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
        ScriptedProvider::new(vec![Script::Events(tool_call("Echo", json!({"v": 1})))]),
        vec![Arc::new(EchoTool { read_only: true })],
        Arc::new(NoHost),
    )
}

async fn stopped_turn(cfg: TurnConfig, hung: Arc<Hung>, text: &str) {
    let mailbox = configured(cfg, vec![]);
    let (mut state, mut events) = mailbox.attach().await.unwrap();
    mailbox.submit(IntentId::mint(), input(text));
    bounded(hung.entered.notified()).await;
    let interrupted_at = tokio::time::Instant::now();
    mailbox.interrupt(IntentId::mint(), InterruptScope::Head);
    let frames = bounded(frames_until(&mut events, &mut state, turn_completed)).await;
    assert!(tokio::time::Instant::now() - interrupted_at <= GRACE);
    for frame in frames {
        if let Event::ItemStarted { item } = frame.event {
            assert_eq!(
                state.transcript.get(&item.id).unwrap().status,
                ItemStatus::Interrupted
            );
        }
    }
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

#[tokio::test(start_paused = true)]
async fn gate_fifo_keeps_prose_command_prose_in_arrival_order() {
    let hung = Hung::at(HookPoint::Submit);
    let provider = ScriptedProvider::new(vec![
        Script::Hang(vec![ModelEvent::TextStart { id: "first".into() }]),
        Script::Events(text("second")),
    ]);
    let mut cfg = config(provider, vec![], Arc::new(NoHost));
    cfg.hooks = HookSet::fixed(vec![hung.clone()]);
    let command =
        ScriptedCommand::new("echo", false, Ok(CommandOutcome::Applied { message: None }));
    let mailbox = configured(cfg, vec![command.clone()]);
    let (mut state, mut events) = mailbox.attach().await.unwrap();
    let blocked = IntentId::mint();
    mailbox.submit(blocked.clone(), input("hang"));
    bounded(hung.entered.notified()).await;
    let first = IntentId::mint();
    let middle = IntentId::mint();
    let last = IntentId::mint();
    mailbox.submit(first.clone(), input("first"));
    mailbox.submit(middle.clone(), input("/echo middle"));
    mailbox.submit(last.clone(), input("last"));
    mailbox.interrupt(IntentId::mint(), InterruptScope::Head);
    rejected(&mut events, &mut state, &blocked).await;
    let frames = bounded(frames_until(
        &mut events,
        &mut state,
        |frame| matches!(&frame.event, Event::IntentAck { intent, .. } if intent == &last),
    ))
    .await;
    let acks: Vec<_> = frames
        .iter()
        .filter_map(|frame| match &frame.event {
            Event::IntentAck { intent, .. }
                if [first.clone(), middle.clone(), last.clone()].contains(intent) =>
            {
                Some(intent.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(acks, [first, middle.clone(), last.clone()]);
    mailbox.interrupt(IntentId::mint(), InterruptScope::Head);
    bounded(drive(&mut events, &mut state, turn_completed)).await;
    let rest = bounded(frames_until(&mut events, &mut state, turn_completed)).await;
    let completed: Vec<_> = rest
        .iter()
        .filter_map(|frame| match &frame.event {
            Event::IntentAck {
                intent,
                outcome: IntentOutcome::Applied { .. } | IntentOutcome::TurnStarted { .. },
            } => Some(intent.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(completed, [middle, last]);
    assert_eq!(command.calls(), ["middle"]);
    assert_eq!(user_texts(&state), ["first", "last"]);
    mailbox.close(CloseReason::Client);
    bounded(mailbox.wait_closed()).await;
}

#[tokio::test(start_paused = true)]
async fn close_joins_a_hung_command_within_the_grace() {
    let hung = Hung::at(HookPoint::Submit);
    let mailbox = configured(tool_config(), vec![hung.clone()]);
    drop(mailbox.attach().await.unwrap());
    mailbox.submit(IntentId::mint(), input("/hang"));
    bounded(hung.entered.notified()).await;
    mailbox.close(CloseReason::Client);
    bounded(mailbox.wait_closed()).await;
}

#[tokio::test(start_paused = true)]
async fn close_rejects_the_gate_head_and_every_waiting_input() {
    let hung = Hung::at(HookPoint::Submit);
    let mut cfg = tool_config();
    cfg.hooks = HookSet::fixed(vec![hung.clone()]);
    let mailbox = configured(cfg, vec![]);
    let (_, mut events) = mailbox.attach().await.unwrap();
    let head = IntentId::mint();
    let tail = IntentId::mint();
    mailbox.submit(head.clone(), input("hang"));
    bounded(hung.entered.notified()).await;
    mailbox.submit(tail.clone(), input("tail"));
    mailbox.close(CloseReason::Client);
    bounded(mailbox.wait_closed()).await;
    let frames: Vec<_> = bounded(events.by_ref().collect()).await;
    let rejected: Vec<_> = frames
        .iter()
        .filter_map(|frame| match &frame.event {
            Event::IntentAck {
                intent,
                outcome: IntentOutcome::Rejected { error },
            } if error.code == ErrorCode::SessionClosed => Some(intent.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(rejected, [head, tail]);
}

#[tokio::test(start_paused = true)]
async fn a_turn_ending_during_submit_does_not_let_prose_overtake_a_command() {
    let slow = Arc::new(crate::test_support::owned::SlowSubmit::default());
    let provider = ScriptedProvider::new(vec![
        Script::Hang(vec![ModelEvent::TextStart { id: "first".into() }]),
        Script::Events(text("last")),
    ]);
    let mut cfg = config(provider, vec![], Arc::new(NoHost));
    cfg.hooks = HookSet::fixed(vec![slow.clone()]);
    let command =
        ScriptedCommand::new("echo", false, Ok(CommandOutcome::Applied { message: None }));
    let mailbox = configured(cfg, vec![command.clone()]);
    let (mut state, mut events) = mailbox.attach().await.unwrap();
    mailbox.submit(IntentId::mint(), input("first"));
    bounded(drive(&mut events, &mut state, |f| {
        matches!(f.event, Event::ItemStarted { .. })
    }))
    .await;
    let middle = IntentId::mint();
    mailbox.submit(middle.clone(), input("/echo middle"));
    bounded(drive(&mut events, &mut state, |f| {
        matches!(&f.event,
        Event::IntentAck { intent, outcome: IntentOutcome::Queued { .. } } if intent == &middle)
    }))
    .await;
    let last = IntentId::mint();
    mailbox.submit(last.clone(), input("slow"));
    bounded(slow.entered.notified()).await;
    mailbox.interrupt(IntentId::mint(), InterruptScope::Head);
    bounded(drive(&mut events, &mut state, turn_completed)).await;
    slow.release.notify_one();
    let frames = bounded(frames_until(&mut events, &mut state, turn_completed)).await;
    let applied: Vec<_> = frames
        .iter()
        .filter_map(|frame| match &frame.event {
            Event::IntentAck {
                intent,
                outcome: IntentOutcome::Applied { .. } | IntentOutcome::TurnStarted { .. },
            } => Some(intent.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(applied, [middle, last]);
    assert_eq!(command.calls(), ["middle"]);
    mailbox.close(CloseReason::Client);
    bounded(mailbox.wait_closed()).await;
}

struct HungLink(Arc<Hung>);
#[async_trait::async_trait]
impl Interceptor for HungLink {
    fn id(&self) -> &str {
        "hung-link"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher::at(bingo_sdk::catalogue::TOOL_CHECK)
    }
    async fn tool_check(
        &self,
        _: &HookContext,
        _: ToolCall,
        _: Next<'_, ToolCall, CheckResult>,
    ) -> Result<CheckResult, KernelError> {
        self.0.entered.notify_one();
        std::future::pending().await
    }
}
#[tokio::test(start_paused = true)]
async fn interrupt_drops_a_hung_tool_check_link() {
    let hung = Hung::at(HookPoint::BeforeTool);
    let mut cfg = tool_config();
    cfg.hooks.links.push(crate::chain::Link {
        external: None,
        tier: Tier::Full,
        interceptor: Arc::new(HungLink(hung.clone())),
    });
    stopped_turn(cfg, hung, "go").await;
}
