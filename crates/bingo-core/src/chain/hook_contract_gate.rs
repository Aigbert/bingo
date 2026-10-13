use super::*;
use crate::test_support::EchoTool;
use std::sync::Mutex;

struct Before(HookOutcome, Arc<Mutex<Vec<String>>>, &'static str);
#[async_trait]
impl Hook for Before {
    fn id(&self) -> &str {
        self.2
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher::default()
    }
    async fn before_tool(&self, _: &mut ToolCall, _: &HookContext) -> HookOutcome {
        self.1.lock().unwrap().push(self.2.into());
        self.0.clone()
    }
}
struct Deny;
#[async_trait]
impl PermissionPolicy for Deny {
    fn id(&self) -> &str {
        "deny"
    }
    async fn decide(&self, _: PolicyInput<'_>) -> Decision {
        Decision::Deny {
            reason: Reason::Default,
        }
    }
}
fn cx() -> HookContext {
    HookContext {
        session: SessionId::from_raw("s"),
        turn: None,
        cwd: "/tmp".into(),
        provider: None,
        model: None,
        host: bingo_sdk::testing::NoHost::handle(),
    }
}
async fn before(outcome: HookOutcome, policy: &dyn PermissionPolicy) -> (Gated, Vec<String>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let hooks: Vec<Arc<dyn Hook>> = vec![
        Arc::new(Before(outcome, seen.clone(), "first")),
        Arc::new(Before(
            HookOutcome::Deny {
                reason: "second".into(),
            },
            seen.clone(),
            "second",
        )),
    ];
    let cx = cx();
    let item = ItemId::from_raw("i");
    let result = gate_call(
        GateInput {
            session: &cx.session,
            cwd: &cx.cwd,
            item: &item,
            call: ToolCall {
                call_id: "c".into(),
                name: "Echo".into(),
                input: serde_json::json!({}),
            },
            tool: Some(Arc::new(EchoTool { read_only: true })),
            policy,
            hooks: &hooks,
            hook_cx: &cx,
        },
        &crate::test_support::NoHost,
    )
    .await;
    let seen = seen.lock().unwrap().clone();
    (result, seen)
}
#[tokio::test]
async fn before_deny_ends_the_run() {
    refused(HookOutcome::Deny {
        reason: "no".into(),
    })
    .await;
}
#[tokio::test]
async fn before_block_ends_the_run() {
    refused(HookOutcome::Block {
        reason: "no".into(),
    })
    .await;
}
async fn refused(outcome: HookOutcome) {
    let (result, seen) = before(outcome, &DefaultPolicy).await;
    assert_eq!(seen, ["first"]);
    assert!(
        matches!(result.gate, Gate::Denied { message } if message == "Denied by hook first: no")
    );
}
#[tokio::test]
async fn before_ask_ends_hooks_but_the_policy_can_still_deny() {
    let (result, seen) = before(
        HookOutcome::Ask {
            reason: "ask".into(),
        },
        &Deny,
    )
    .await;
    assert_eq!(seen, ["first"]);
    assert!(
        matches!(result.gate, Gate::Denied { message } if message == "Permission denied (default)")
    );
    assert!(result.receipt.is_none(), "the person was never asked");
}
#[tokio::test]
async fn before_redirect_continues() {
    let (result, seen) = before(
        HookOutcome::Redirect {
            session: SessionId::from_raw("other"),
        },
        &DefaultPolicy,
    )
    .await;
    assert_eq!(seen, ["first", "second"]);
    assert!(matches!(result.gate, Gate::Denied { message } if message.contains("second")));
}
#[test]
fn hooks_match_exact_names_and_prefixes() {
    let matcher = |tool: &str| HookMatcher {
        points: vec![HookPoint::BeforeTool],
        tool: Some(tool.into()),
    };
    assert!(hook_applies(
        &matcher("Echo"),
        HookPoint::BeforeTool,
        Some("Echo")
    ));
    assert!(!hook_applies(
        &matcher("Echo"),
        HookPoint::BeforeTool,
        Some("EchoMore")
    ));
    assert!(hook_applies(
        &matcher("Echo*"),
        HookPoint::BeforeTool,
        Some("EchoMore")
    ));
}
