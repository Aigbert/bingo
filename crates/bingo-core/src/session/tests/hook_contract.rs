use super::*;
use async_trait::async_trait;
use std::sync::Mutex;

struct Submit {
    id: &'static str,
    outcome: HookOutcome,
    seen: Arc<Mutex<Vec<String>>>,
}
#[async_trait]
impl Hook for Submit {
    fn id(&self) -> &str {
        self.id
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher::default()
    }
    async fn on_submit(&self, input: &mut Input, _: &HookContext) -> HookOutcome {
        if let Input::Text { text, .. } = input {
            self.seen.lock().unwrap().push(text.clone());
            text.push_str(self.id);
        }
        self.outcome.clone()
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
async fn submit(outcome: HookOutcome) -> (Verdict, Vec<String>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let hooks = HookSet::fixed(vec![
        Arc::new(Submit {
            id: "a",
            outcome,
            seen: seen.clone(),
        }),
        Arc::new(Submit {
            id: "b",
            outcome: HookOutcome::Continue,
            seen: seen.clone(),
        }),
    ]);
    let result = submit_hooks(
        Input::text("x", Origin::surface("test")),
        hooks,
        cx(),
        crate::chain::Report::default(),
    )
    .await;
    let seen = seen.lock().unwrap().clone();
    (result, seen)
}
#[tokio::test]
async fn submit_rewrites_in_order() {
    let (result, seen) = submit(HookOutcome::Continue).await;
    assert_eq!(seen, ["x", "xa"]);
    assert!(matches!(result, Verdict::Prose(Input::Text { text, .. }) if text == "xab"));
}
#[tokio::test]
async fn ask_at_submit_continues() {
    let (result, seen) = submit(HookOutcome::Ask {
        reason: "ask".into(),
    })
    .await;
    assert_eq!(seen.len(), 2);
    assert!(matches!(result, Verdict::Prose(_)));
}
#[tokio::test]
async fn submit_deny_ends_the_run() {
    refused(HookOutcome::Deny {
        reason: "no".into(),
    })
    .await;
}
#[tokio::test]
async fn submit_block_ends_the_run() {
    refused(HookOutcome::Block {
        reason: "no".into(),
    })
    .await;
}
async fn refused(outcome: HookOutcome) {
    let (result, seen) = submit(outcome).await;
    assert_eq!(seen, ["x"]);
    assert!(matches!(
        result,
        Verdict::Rejected(KernelError {
            code: ErrorCode::PermissionDenied,
            ..
        })
    ));
}
#[tokio::test]
async fn submit_redirect_ends_the_run() {
    let (result, seen) = submit(HookOutcome::Redirect {
        session: SessionId::from_raw("elsewhere"),
    })
    .await;
    assert_eq!(seen, ["x"]);
    assert!(
        matches!(result, Verdict::Redirected { to, .. } if to == SessionId::from_raw("elsewhere"))
    );
}
