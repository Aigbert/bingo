use super::*;
use std::collections::HashMap;
use tokio::sync::oneshot;

type Ask = (InteractionKind, oneshot::Sender<Answer>);
type Releases = Arc<Mutex<HashMap<usize, oneshot::Receiver<()>>>>;

struct Person {
    recorded: RecordingHost,
    asks: mpsc::UnboundedSender<Ask>,
    events: mpsc::UnboundedSender<Event>,
}

#[async_trait]
impl TurnHost for Person {
    fn offered(&self, _: Vec<Arc<dyn Tool>>) {}
    fn emit(&self, event: Event) {
        self.recorded.emit(event.clone());
        self.events.send(event).unwrap();
    }
    async fn absorb(&self) -> Vec<(IntentId, Input)> {
        vec![]
    }
    async fn ask(
        &self,
        _: Option<ItemId>,
        kind: InteractionKind,
        _: Vec<AnswerSpec>,
    ) -> Result<crate::gate::Answered, KernelError> {
        let (reply, answer) = oneshot::channel();
        self.asks.send((kind, reply)).unwrap();
        Ok(crate::gate::Answered {
            ask: crate::gate::Asked {
                id: InteractionId::mint(),
                opened_at: Timestamp::now(),
            },
            answer: answer.await.unwrap(),
        })
    }
}

struct Controlled {
    name: String,
    traits: ToolTraits,
    starts: mpsc::UnboundedSender<usize>,
    releases: Releases,
    host: Arc<Person>,
}

#[async_trait]
impl Tool for Controlled {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name.clone(),
            ..EchoTool { read_only: true }.spec()
        }
    }
    fn traits(&self, _: &serde_json::Value) -> ToolTraits {
        self.traits
    }
    async fn call(
        &self,
        input: serde_json::Value,
        cx: &ToolContext,
    ) -> Result<ToolOutput, ToolError> {
        assert!(self.host.recorded.events().iter().any(|event| matches!(event,
            Event::ItemUpdated { item } if item.id == cx.item && item.status == ItemStatus::Running)));
        let index = input["v"].as_u64().unwrap() as usize;
        let release = self.releases.lock().unwrap().remove(&index).unwrap();
        self.starts.send(index).unwrap();
        release.await.unwrap();
        Ok(ToolOutput::text(index.to_string()))
    }
}

fn controlled_tools(
    definitions: &[(&str, bool, bool)],
    starts: mpsc::UnboundedSender<usize>,
    releases: Releases,
    host: &Arc<Person>,
) -> Vec<Arc<dyn Tool>> {
    definitions
        .iter()
        .map(|(name, read_only, concurrency_safe)| {
            Arc::new(Controlled {
                name: (*name).into(),
                traits: ToolTraits {
                    read_only: *read_only,
                    concurrency_safe: *concurrency_safe,
                    trusted: true,
                    ..ToolTraits::default()
                },
                starts: starts.clone(),
                releases: releases.clone(),
                host: host.clone(),
            }) as Arc<dyn Tool>
        })
        .collect()
}

struct Rig {
    host: Arc<Person>,
    asks: mpsc::UnboundedReceiver<Ask>,
    starts: mpsc::UnboundedReceiver<usize>,
    events: mpsc::UnboundedReceiver<Event>,
    releases: HashMap<usize, oneshot::Sender<()>>,
    cancel: CancellationToken,
    task: tokio::task::JoinHandle<TurnOutcome>,
    provider: Arc<ScriptedProvider>,
}

impl Rig {
    fn new(
        names: &[&str],
        definitions: &[(&str, bool, bool)],
        policy: Option<Arc<dyn PermissionPolicy>>,
        hooks: HookSet,
    ) -> Self {
        let (asks_tx, asks) = mpsc::unbounded_channel();
        let (events_tx, events) = mpsc::unbounded_channel();
        let host = Arc::new(Person {
            recorded: RecordingHost::new(),
            asks: asks_tx,
            events: events_tx,
        });
        let (starts_tx, starts) = mpsc::unbounded_channel();
        let mut releases = HashMap::new();
        let receivers = Arc::new(Mutex::new(HashMap::new()));
        for index in 0..names.len() {
            let (send, receive) = oneshot::channel();
            releases.insert(index, send);
            receivers.lock().unwrap().insert(index, receive);
        }
        let tools = controlled_tools(definitions, starts_tx, receivers, &host);
        let provider = ScriptedProvider::new(vec![
            Script::Events(calls(names)),
            Script::Events(text("done")),
        ]);
        let mut cfg = config(provider.clone(), tools);
        if let Some(policy) = policy {
            cfg.policy = policy;
        }
        cfg.hooks = hooks;
        let cancel = CancellationToken::new();
        let task = tokio::spawn({
            let host = host.clone();
            let cancel = cancel.clone();
            async move {
                run_turn(
                    &cfg,
                    TurnRun::new(
                        TurnId::from_raw("trn_1"),
                        &transcript(&history("go")),
                        cancel,
                        TurnKind::Respond,
                    ),
                    host.as_ref(),
                )
                .await
            }
        });
        Self {
            host,
            asks,
            starts,
            events,
            releases,
            cancel,
            task,
            provider,
        }
    }
    fn release(&mut self, index: usize) {
        self.releases.remove(&index).unwrap().send(()).unwrap();
    }
    fn status(&self, index: usize) -> ItemStatus {
        let transcript = self
            .host
            .recorded
            .events()
            .iter()
            .fold(Transcript::new(), |mut t, e| {
                t.apply(e);
                t
            });
        transcript
            .items()
            .iter()
            .find(|item| {
                matches!(&item.body,
            ItemBody::ToolCall { call_id, .. } if call_id == &format!("c{index}"))
            })
            .unwrap()
            .status
    }
    async fn landed(&mut self) -> String {
        loop {
            if let Event::ItemCompleted { item } = self
                .events
                .recv()
                .await
                .expect("turn ended before a call landed")
                && let ItemBody::ToolCall { call_id, .. } = item.body
            {
                return call_id;
            }
        }
    }
    async fn answer(&mut self, tool: &str, answer: Answer) {
        let (kind, reply) = self.asks.recv().await.unwrap();
        assert!(matches!(kind, InteractionKind::Permission { tool: name, .. } if name == tool));
        reply.send(answer).unwrap();
    }
}

#[tokio::test]
async fn read_lands_before_edit_asks_and_edit_lands_before_bash_is_reached() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut rig = Rig::new(
        &["Read", "Edit", "Bash"],
        &[
            ("Read", true, true),
            ("Edit", false, false),
            ("Bash", false, false),
        ],
        None,
        HookSet::fixed(vec![OrderingHook::new("check", &seen)]),
    );
    assert_eq!(rig.starts.recv().await, Some(0));
    assert_eq!(rig.status(1), ItemStatus::Pending);
    assert_eq!(rig.status(2), ItemStatus::Pending);
    assert!(rig.asks.try_recv().is_err());
    rig.release(0);
    rig.answer("Edit", Answer::AllowOnce).await;
    assert_eq!(rig.status(0), ItemStatus::Completed);
    assert_eq!(seen.lock().unwrap().len(), 2);
    assert_eq!(rig.starts.recv().await, Some(1));
    assert_eq!(rig.status(2), ItemStatus::Pending);
    rig.release(1);
    rig.answer("Bash", Answer::AllowOnce).await;
    assert_eq!(rig.status(1), ItemStatus::Completed);
    assert_eq!(rig.starts.recv().await, Some(2));
    rig.release(2);
    assert_eq!(rig.task.await.unwrap().status, TurnStatus::Completed);
}

#[tokio::test]
async fn each_safe_call_lands_at_once_and_the_model_keeps_input_order() {
    let mut rig = Rig::new(
        &["Read", "Read"],
        &[("Read", true, true)],
        None,
        HookSet::default(),
    );
    assert_eq!(rig.starts.recv().await, Some(0));
    assert_eq!(rig.starts.recv().await, Some(1));
    rig.release(1);
    assert_eq!(rig.landed().await, "c1");
    assert_eq!(rig.status(0), ItemStatus::Running);
    rig.release(0);
    assert_eq!(rig.landed().await, "c0");
    assert_eq!(rig.task.await.unwrap().status, TurnStatus::Completed);
    let results: Vec<_> = rig.provider.requests()[1]
        .messages
        .iter()
        .flat_map(|m| &m.parts)
        .filter_map(|p| match p {
            ContentPart::ToolResult { tool_use_id, .. } => Some(tool_use_id.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(results, ["c0", "c1"]);
}

#[tokio::test]
async fn a_landing_frees_one_slot_without_waiting_for_the_window() {
    let mut rig = Rig::new(
        &["Read"; 11],
        &[("Read", true, true)],
        None,
        HookSet::default(),
    );
    for index in 0..10 {
        assert_eq!(rig.starts.recv().await, Some(index));
    }
    assert_eq!(rig.status(10), ItemStatus::Pending);
    assert!(rig.starts.try_recv().is_err());
    rig.release(7);
    assert_eq!(rig.landed().await, "c7");
    assert_eq!(rig.starts.recv().await, Some(10));
    for index in 0..11 {
        if index != 7 {
            rig.release(index);
        }
    }
    assert_eq!(rig.task.await.unwrap().status, TurnStatus::Completed);
}

#[tokio::test]
async fn esc_drops_parallel_calls_and_never_reaches_the_call_after_a_writer() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut rig = Rig::new(
        &["Read", "Read", "Edit", "Bash"],
        &[
            ("Read", true, true),
            ("Edit", false, false),
            ("Bash", false, false),
        ],
        None,
        HookSet::fixed(vec![OrderingHook::new("check", &seen)]),
    );
    assert_eq!(rig.starts.recv().await, Some(0));
    assert_eq!(rig.starts.recv().await, Some(1));
    rig.release(1);
    assert_eq!(rig.landed().await, "c1");
    rig.cancel.cancel();
    let out = rig.task.await.unwrap();
    let statuses: Vec<_> = out
        .items
        .iter()
        .filter_map(|i| matches!(i.body, ItemBody::ToolCall { .. }).then_some(i.status))
        .collect();
    assert_eq!(
        statuses,
        [
            ItemStatus::Interrupted,
            ItemStatus::Completed,
            ItemStatus::Interrupted,
            ItemStatus::Interrupted
        ]
    );
    assert_eq!(seen.lock().unwrap().len(), 3);
    assert!(rig.asks.try_recv().is_err());
    assert!(
        out.items
            .iter()
            .all(|i| !matches!(i.body, ItemBody::PermissionReceipt { .. }))
    );
}

struct SessionPolicy(std::sync::atomic::AtomicBool);
#[async_trait]
impl PermissionPolicy for SessionPolicy {
    fn id(&self) -> &str {
        "session-fixture"
    }
    async fn decide(&self, input: PolicyInput<'_>) -> Decision {
        if input.call.name == "Deny" {
            Decision::Deny {
                reason: Reason::Default,
            }
        } else if self.0.load(std::sync::atomic::Ordering::Acquire) || input.traits.read_only {
            Decision::Allow {
                reason: Reason::Rule {
                    rule: "Edit".into(),
                    source: Some(bingo_sdk::policy::RuleSource::Session),
                },
            }
        } else {
            Decision::Ask {
                reason: Reason::Default,
                scope: Some("Edit".into()),
            }
        }
    }
    async fn on_verdict(&self, _: PolicyInput<'_>, verdict: &Verdict) {
        if matches!(verdict, Verdict::Allow { scope: Some(_) }) {
            self.0.store(true, std::sync::atomic::Ordering::Release);
        }
    }
}

#[tokio::test]
async fn yes_for_the_session_answers_the_next_edit_in_the_same_round() {
    let mut rig = Rig::new(
        &["Edit", "Edit"],
        &[("Edit", false, false)],
        Some(Arc::new(SessionPolicy(false.into()))),
        HookSet::default(),
    );
    rig.answer(
        "Edit",
        Answer::AllowSession {
            scope: "Edit".into(),
        },
    )
    .await;
    assert_eq!(rig.starts.recv().await, Some(0));
    rig.release(0);
    assert_eq!(rig.starts.recv().await, Some(1));
    assert!(rig.asks.try_recv().is_err());
    rig.release(1);
    let out = rig.task.await.unwrap();
    assert_eq!(out.status, TurnStatus::Completed);
    assert_eq!(
        out.items
            .iter()
            .filter(|item| matches!(item.body, ItemBody::PermissionReceipt { .. }))
            .count(),
        1
    );
    assert!(out.items.iter().any(|item| matches!(&item.body,
        ItemBody::ToolCall { call_id, allowed_by: Some(reason), .. }
            if call_id == "c1" && matches!(reason.as_ref(), Reason::Rule { .. }))));
}

#[tokio::test]
async fn a_denial_lands_without_waiting_and_does_not_stop_the_walk() {
    let mut rig = Rig::new(
        &["Read", "Deny", "Read"],
        &[("Read", true, true), ("Deny", false, false)],
        Some(Arc::new(SessionPolicy(false.into()))),
        HookSet::default(),
    );
    assert_eq!(rig.starts.recv().await, Some(0));
    assert_eq!(rig.starts.recv().await, Some(2));
    assert_eq!(rig.landed().await, "c1");
    assert_eq!(rig.status(0), ItemStatus::Running);
    assert_eq!(rig.status(1), ItemStatus::Failed);
    rig.release(0);
    rig.release(2);
    assert_eq!(rig.task.await.unwrap().status, TurnStatus::Completed);
    assert!(!rig.host.recorded.events().iter().any(|event| matches!(event,
        Event::ItemUpdated { item } if matches!(&item.body, ItemBody::ToolCall { call_id, .. } if call_id == "c1")
            && item.status == ItemStatus::Running)));
}

struct AfterOrder(Arc<Mutex<Vec<String>>>);
#[async_trait]
impl Hook for AfterOrder {
    fn id(&self) -> &str {
        "after-order"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![HookPoint::AfterTool],
            tool: None,
        }
    }
    async fn after_tool(&self, call: &ToolCall, _: &ToolOutput, _: &HookContext) -> HookOutcome {
        self.0.lock().unwrap().push(call.call_id.clone());
        HookOutcome::Continue
    }
}

#[tokio::test]
async fn after_hooks_see_the_whole_round_once_in_input_order() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut rig = Rig::new(
        &["Read", "Read"],
        &[("Read", true, true)],
        None,
        HookSet::fixed(vec![Arc::new(AfterOrder(seen.clone()))]),
    );
    for index in 0..2 {
        assert_eq!(rig.starts.recv().await, Some(index));
    }
    rig.release(1);
    assert_eq!(rig.landed().await, "c1");
    assert!(seen.lock().unwrap().is_empty());
    rig.release(0);
    assert_eq!(rig.task.await.unwrap().status, TurnStatus::Completed);
    assert_eq!(*seen.lock().unwrap(), ["c0", "c1"]);
}

struct HeldCheck(mpsc::UnboundedSender<()>);
#[async_trait]
impl Hook for HeldCheck {
    fn id(&self) -> &str {
        "held-check"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![HookPoint::BeforeTool],
            tool: None,
        }
    }
    async fn before_tool(&self, call: &mut ToolCall, _: &HookContext) -> HookOutcome {
        if call.input["v"] == 1 {
            self.0.send(()).unwrap();
            std::future::pending::<()>().await;
        }
        HookOutcome::Continue
    }
}

#[tokio::test]
async fn a_landing_is_reported_while_a_check_waits_and_esc_drops_that_check() {
    let (checking, mut checks) = mpsc::unbounded_channel();
    let mut rig = Rig::new(
        &["Read", "Read", "Read"],
        &[("Read", true, true)],
        None,
        HookSet::fixed(vec![Arc::new(HeldCheck(checking))]),
    );
    assert_eq!(rig.starts.recv().await, Some(0));
    checks.recv().await.unwrap();
    rig.release(0);
    assert_eq!(rig.landed().await, "c0");
    assert_eq!(rig.status(1), ItemStatus::Pending);
    rig.cancel.cancel();
    let out = rig.task.await.unwrap();
    assert!(matches!(out.status, TurnStatus::Interrupted { .. }));
    assert!(rig.starts.try_recv().is_err());
}
