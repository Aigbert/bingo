use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

struct PolicyDecision(Decision, AtomicUsize);
#[async_trait]
impl PermissionPolicy for PolicyDecision {
    fn id(&self) -> &str {
        "missing-policy"
    }
    async fn decide(&self, _: PolicyInput<'_>) -> Decision {
        self.1.fetch_add(1, Ordering::SeqCst);
        self.0.clone()
    }
}
struct Before(HookOutcome);
#[async_trait]
impl Hook for Before {
    fn id(&self) -> &str {
        "guard"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher::default()
    }
    async fn before_tool(&self, _: &mut ToolCall, _: &HookContext) -> HookOutcome {
        self.0.clone()
    }
}
struct AllowLink;
#[async_trait]
impl Interceptor for AllowLink {
    fn id(&self) -> &str {
        "allow-missing"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher::default()
    }
    async fn tool_check(
        &self,
        _: &HookContext,
        call: ToolCall,
        next: Next<'_, ToolCall, CheckResult>,
    ) -> Result<CheckResult, KernelError> {
        let mut checked = next.run(call).await?;
        assert!(matches!(checked.decision, Decision::Deny { .. }));
        checked.decision = Decision::Allow {
            reason: Reason::Default,
        };
        Ok(checked)
    }
}

#[tokio::test]
async fn missing_tools_settle_after_hooks_without_policy_or_person() {
    let rule = Reason::Rule {
        rule: "Missing".into(),
        source: None,
    };
    let ask = Decision::Ask {
        scope: None,
        reason: Reason::Default,
    };
    for (name, decision, hook, full) in [
        (
            "rule allow",
            Decision::Allow {
                reason: rule.clone(),
            },
            HookOutcome::Continue,
            false,
        ),
        (
            "mode-allow",
            Decision::Allow {
                reason: Reason::Mode {
                    mode: "mode-allow".into(),
                },
            },
            HookOutcome::Continue,
            false,
        ),
        ("policy ask", ask.clone(), HookOutcome::Continue, false),
        (
            "hook ask",
            ask.clone(),
            HookOutcome::Ask {
                reason: "check".into(),
            },
            false,
        ),
        (
            "hook deny",
            ask.clone(),
            HookOutcome::Deny {
                reason: "keep this".into(),
            },
            false,
        ),
        (
            "rule deny",
            Decision::Deny { reason: rule },
            HookOutcome::Continue,
            false,
        ),
        ("full allow", ask, HookOutcome::Continue, true),
    ] {
        missing_case(name, decision, hook, full).await;
    }
}

async fn missing_case(name: &str, decision: Decision, hook: HookOutcome, full: bool) {
    let message = if matches!(hook, HookOutcome::Deny { .. }) {
        "Denied by hook guard: keep this"
    } else {
        "tool not found: Missing"
    };
    let policy = PolicyDecision(decision, AtomicUsize::new(0));
    let hooks: Vec<Arc<dyn Hook>> = vec![Arc::new(Before(hook))];
    let links = full
        .then(|| crate::chain::Link {
            plugin: "fixture".into(),
            placement: bingo_sdk::plugin::Placement::Outer,
            interceptor: Arc::new(AllowLink),
            external: None,
            tier: Tier::Full,
        })
        .into_iter()
        .collect::<Vec<_>>();
    let cx = crate::test_support::hook_context("/work");
    let item = ItemId::from_raw("i");
    let input = GateInput {
        session: &cx.session,
        cwd: &cx.cwd,
        item: &item,
        call: ToolCall {
            name: "Missing".into(),
            call_id: "c".into(),
            input: json!({}),
        },
        tool: None,
        policy: &policy,
        links: &links,
        hooks: &hooks,
        hook_cx: &cx,
    };
    let Decided::Settled(changed) = check(&input, true).await else {
        panic!("{name}: missing replacement asked");
    };
    assert_missing(name, &changed, message);
    let person = Person::default();
    let gated = gate_call(input, &person).await;
    assert_missing(name, &gated, message);
    assert!(
        person.asks.lock().unwrap().is_empty(),
        "{name}: asked a person"
    );
    assert_eq!(
        policy.1.load(Ordering::SeqCst),
        0,
        "{name}: consulted policy"
    );
}
fn assert_missing(name: &str, gated: &Gated, message: &str) {
    assert_eq!(
        gated.gate,
        Gate::Denied {
            message: message.into()
        },
        "{name}"
    );
    assert!(gated.allowed_by.is_none(), "{name}: allowed_by");
    assert!(gated.receipt.is_none(), "{name}: receipt");
}
