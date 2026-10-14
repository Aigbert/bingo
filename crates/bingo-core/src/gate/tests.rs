use super::*;
use serde_json::{Value, json};
use std::{collections::VecDeque, sync::Mutex};

struct Shell;
#[async_trait]
impl Tool for Shell {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
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
impl Prompter for Person {
    async fn ask(
        &self,
        kind: InteractionKind,
        specs: Vec<AnswerSpec>,
    ) -> Result<Answer, KernelError> {
        self.asks.lock().unwrap().push((kind, specs));
        Ok(self
            .replies
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Answer::Deny { feedback: None }))
    }
}
async fn run(command: &str, replies: Vec<Answer>) -> (Gated, Person) {
    let person = Person {
        replies: Mutex::new(replies.into()),
        ..Default::default()
    };
    let cx = HookContext {
        item: None,
        session: SessionId::from_raw("s"),
        turn: None,
        cwd: "/work".into(),
        provider: None,
        model: None,
        host: bingo_sdk::testing::NoHost::handle(),
    };
    let result = gate_call(
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
            hooks: &[],
            hook_cx: &cx,
        },
        &person,
    )
    .await;
    (result, person)
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
        asks[0].0,
        InteractionKind::Permission {
            caution: Some(_),
            session_scope: None,
            ..
        }
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
