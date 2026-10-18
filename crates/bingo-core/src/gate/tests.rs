use super::*;
use serde_json::{Value, json};
use std::{collections::VecDeque, sync::Mutex};

struct Shell;
#[async_trait]
impl Tool for Shell {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            look: None,
            name: "Bash".into(),
            description: "fixture".into(),
            input_schema: json!({}),
            meta: Default::default(),
        }
    }
    fn traits(&self, input: &Value) -> ToolTraits {
        ToolTraits {
            trusted: true,
            destructive: input["command"] == "rm -rf ~",
            ..Default::default()
        }
    }
    fn subjects(&self, input: &Value, _: &Path) -> Vec<Subject> {
        vec![Subject::Command {
            command: input["command"].as_str().unwrap_or_default().into(),
        }]
    }
    fn preview(&self, input: &Value, _: &Path) -> Option<Preview> {
        Some(Preview::Command {
            command: input["command"].as_str().unwrap_or_default().into(),
            cwd: "/work".into(),
            description: None,
            stakes: None,
        })
    }
    async fn call(&self, _: Value, _: &ToolContext) -> Result<ToolOutput, ToolError> {
        unreachable!("gate tests never execute")
    }
}
struct Policy;
#[async_trait]
impl PermissionPolicy for Policy {
    fn allows_replacement(&self, input: PolicyInput<'_>) -> bool {
        input.traits.trusted
    }
    fn id(&self) -> &str {
        "fixture"
    }
    fn caution(&self, input: PolicyInput<'_>) -> Option<Caution> {
        (input.call.input["command"] == "git push --force origin dev").then(|| Caution {
            reason: "rewrites remote history".into(),
            act: Some("force-push".into()),
        })
    }
    async fn decide(&self, input: PolicyInput<'_>) -> Decision {
        match input.call.input["command"].as_str().unwrap_or_default() {
            "cargo test" => Decision::Allow {
                reason: Reason::Default,
            },
            "cargo test; rm -rf ~" => Decision::Deny {
                reason: Reason::Default,
            },
            _ => Decision::Ask {
                reason: Reason::Default,
                scope: Some("Bash(cargo:*)".into()),
            },
        }
    }
}
#[derive(Default)]
struct Person {
    replies: Mutex<VecDeque<Answer>>,
    asks: Mutex<Vec<(InteractionKind, Vec<AnswerSpec>)>>,
}
#[async_trait]
impl GatePrompter for Person {
    async fn ask(
        &self,
        kind: InteractionKind,
        specs: Vec<AnswerSpec>,
    ) -> Result<Answered, KernelError> {
        self.asks.lock().unwrap().push((kind, specs));
        Ok(Answered {
            ask: Asked {
                id: InteractionId::mint(),
                opened_at: jiff::Timestamp::now(),
            },
            answer: self
                .replies
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Answer::Deny { feedback: None }),
        })
    }
}
async fn run(command: &str, replies: Vec<Answer>) -> (Gated, Person) {
    with_hooks(command, replies, &[]).await
}
async fn with_hooks(
    command: &str,
    replies: Vec<Answer>,
    hooks: &[Arc<dyn Hook>],
) -> (Gated, Person) {
    let person = Person {
        replies: Mutex::new(replies.into()),
        ..Default::default()
    };
    let result = through_gate(command, hooks, &person).await;
    (result, person)
}

async fn through_gate(command: &str, hooks: &[Arc<dyn Hook>], person: &dyn GatePrompter) -> Gated {
    let cx = HookContext {
        item: None,
        session: SessionId::from_raw("s"),
        turn: None,
        cwd: "/work".into(),
        provider: None,
        model: None,
        host: bingo_sdk::testing::NoHost::handle(),
    };
    gate_call(
        GateInput {
            session: &cx.session,
            cwd: &cx.cwd,
            item: &ItemId::from_raw("i"),
            call: ToolCall {
                name: "Bash".into(),
                call_id: "c".into(),
                input: json!({"command":command}),
            },
            tool: Some(Arc::new(Shell)),
            policy: &Policy,
            links: &[],
            hooks,
            hook_cx: &cx,
        },
        person,
    )
    .await
}
fn changed(command: &str) -> Answer {
    Answer::AllowChanged {
        input: json!({"command":command}),
    }
}
#[tokio::test]
async fn changed_calls_repeat_policy_and_keep_the_replacement() {
    let (result, person) = run("cargo build", vec![changed("cargo test")]).await;
    assert_eq!(result.gate, Gate::Allowed);
    assert_eq!(result.call.input, json!({"command":"cargo test"}));
    assert_eq!(person.asks.lock().unwrap().len(), 1);
    assert!(matches!(
        result.receipt,
        Some(ItemBody::PermissionReceipt {
            decision: DecisionKind::AllowChanged,
            ..
        })
    ));
}
#[tokio::test]
async fn a_changed_call_that_asks_must_be_approved_again() {
    let (result, person) = run(
        "cargo build",
        vec![changed("cargo check"), Answer::AllowOnce],
    )
    .await;
    assert_eq!(result.gate, Gate::Allowed);
    assert_eq!(person.asks.lock().unwrap().len(), 2);
}
#[tokio::test]
async fn cargo_test_then_rm_cannot_escape_the_denial_by_changing_input() {
    let (result, person) = run(
        "cargo build",
        vec![changed("cargo test; rm -rf ~"), Answer::AllowOnce],
    )
    .await;
    assert!(matches!(result.gate, Gate::Denied { .. }));
    let asks = person.asks.lock().unwrap();
    assert_eq!(asks.len(), 2);
    assert_eq!(asks[1].1, [AnswerSpec::Deny, AnswerSpec::AllowChanged]);
}
#[tokio::test]
async fn a_denied_replacement_can_be_changed_again() {
    let (result, _) = run(
        "cargo build",
        vec![changed("cargo test; rm -rf ~"), changed("cargo test")],
    )
    .await;
    assert_eq!(result.gate, Gate::Allowed);
}
#[tokio::test]
async fn destructive_input_has_caution_and_no_session_answer() {
    let (_, person) = run(
        "rm -rf ~",
        vec![Answer::AllowSession {
            scope: "Bash(cargo:*)".into(),
        }],
    )
    .await;
    let asks = person.asks.lock().unwrap();
    assert!(matches!(
        &asks[0].0,
        InteractionKind::Permission {
            caution: Some(Caution { reason, act: None }),
            session_scope: None,
            ..
        } if reason == "this call can irreversibly destroy data"
    ));
    assert!(!asks[0].1.contains(&AnswerSpec::AllowSession));
}

#[tokio::test]
async fn changed_session_approval_keeps_both_receipt_facts() {
    let (result, _) = run(
        "cargo build",
        vec![
            changed("cargo check"),
            Answer::AllowSession {
                scope: "Bash(cargo:*)".into(),
            },
        ],
    )
    .await;
    let receipt = serde_json::to_value(result.receipt.unwrap()).unwrap();
    assert_eq!(receipt["decision"], "allowChangedSession");
}

struct Rewrite;
#[async_trait]
impl Hook for Rewrite {
    fn id(&self) -> &str {
        "rewrite"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![HookPoint::BeforeTool],
            tool: None,
        }
    }
    async fn before_tool(&self, call: &mut ToolCall, _: &HookContext) -> HookOutcome {
        call.input = json!({"command":"cargo check"});
        HookOutcome::Continue
    }
}
#[tokio::test]
async fn permission_input_is_the_call_after_pre_tool_hooks() {
    let (result, person) =
        with_hooks("cargo build", vec![Answer::AllowOnce], &[Arc::new(Rewrite)]).await;
    let asks = person.asks.lock().unwrap();
    let InteractionKind::Permission { input, .. } = &asks[0].0 else {
        panic!("permission")
    };
    assert_eq!(input, &json!({"command":"cargo check"}));
    assert_eq!(input, &result.call.input);
}

#[tokio::test]
async fn a_policy_caution_keeps_its_act_and_drops_the_session_scope() {
    let (result, person) = run(
        "git push --force origin dev",
        vec![Answer::AllowSession {
            scope: "Bash(cargo:*)".into(),
        }],
    )
    .await;
    assert!(matches!(result.gate, Gate::Denied { .. }));
    let asks = person.asks.lock().unwrap();
    assert!(matches!(&asks[0].0, InteractionKind::Permission {
        caution: Some(Caution { reason, act: Some(act) }), session_scope: None, ..
    } if reason == "rewrites remote history" && act == "force-push"));
    assert!(!asks[0].1.contains(&AnswerSpec::AllowSession));
}

struct Closed;
#[async_trait]
impl GatePrompter for Closed {
    async fn ask(&self, _: InteractionKind, _: Vec<AnswerSpec>) -> Result<Answered, KernelError> {
        Err(KernelError::new(ErrorCode::InteractionClosed, "gone"))
    }
}

#[tokio::test]
async fn a_failed_prompter_closes_the_call_without_a_receipt() {
    let result = through_gate("cargo build", &[], &Closed).await;
    assert_eq!(
        result.gate,
        Gate::Denied {
            message: "Permission interaction closed".into()
        }
    );
    assert!(result.receipt.is_none());
}

mod missing;
