use super::*;
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Wrap(&'static str);
#[async_trait]
impl Interceptor for Wrap {
    fn id(&self) -> &str {
        self.0
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher::default()
    }
    async fn tool_call(
        &self,
        _: &HookContext,
        input: ToolCall,
        next: Next<'_, ToolCall, ToolOutput>,
    ) -> Result<ToolOutput, KernelError> {
        match self.0 {
            "drain" => {
                next.run_then(input, |output| async move {
                    tokio::time::sleep(HOOK_DEADLINE * 3).await;
                    output
                })
                .await
            }
            "answer" => Ok(ToolOutput::text("link")),
            "error-before" => Err(KernelError::new(ErrorCode::Internal, "before")),
            "hang-before" => std::future::pending().await,
            "error-after" => {
                let _ = next.run(input).await?;
                Err(KernelError::new(ErrorCode::Internal, "after"))
            }
            "hang-after" => {
                let _ = next.run(input).await?;
                std::future::pending().await
            }
            _ => next.run(input).await,
        }
    }
    async fn tool_check(
        &self,
        _: &HookContext,
        input: ToolCall,
        next: Next<'_, ToolCall, CheckResult>,
    ) -> Result<CheckResult, KernelError> {
        let mut result = next.run(input).await?;
        result.decision = Decision::Allow {
            reason: Reason::Default,
        };
        Ok(result)
    }
}
fn cx() -> HookContext {
    HookContext {
        item: None,
        session: SessionId::from_raw("s"),
        turn: None,
        cwd: "/tmp".into(),
        provider: None,
        model: None,
        host: bingo_sdk::testing::NoHost::handle(),
    }
}
fn call() -> ToolCall {
    ToolCall {
        name: "Echo".into(),
        call_id: "c".into(),
        input: serde_json::json!({}),
    }
}
fn link(name: &'static str, tier: Tier) -> Link {
    Link {
        placement: bingo_sdk::plugin::Placement::Outer,
        external: Some("process".into()),
        interceptor: Arc::new(Wrap(name)),
        tier,
    }
}
#[tokio::test(start_paused = true)]
async fn a_failed_or_expired_link_never_runs_its_continuation_twice() {
    for name in ["error-before", "error-after", "hang-before", "hang-after"] {
        let count = AtomicUsize::new(0);
        let kernel = Next::new(|_| {
            Box::pin(async {
                count.fetch_add(1, Ordering::SeqCst);
                Ok(ToolOutput::text("kernel"))
            })
        });
        let report = Report::default();
        let result = run::<tool::Call>(
            &[link(name, Tier::Full)],
            kernel,
            &cx(),
            call(),
            report.clone(),
        )
        .await
        .unwrap();
        assert_eq!(result, ToolOutput::text("kernel"));
        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert_eq!(report.take().len(), 1);
    }
}
#[tokio::test]
async fn a_full_link_answers_without_the_kernel_and_receipts_its_name() {
    let kernel = Next::new(|_| Box::pin(async { panic!("the kernel must not run") }));
    let report = Report::default();
    let result = run::<tool::Call>(
        &[link("answer", Tier::Full)],
        kernel,
        &cx(),
        call(),
        report.clone(),
    )
    .await
    .unwrap();
    assert_eq!(result, ToolOutput::text("link"));
    assert!(
        matches!(&report.take()[0], ItemBody::Notice { code, text, .. } if code == "INTERCEPTOR_RECEIPT" && text.contains("answer"))
    );
}
#[tokio::test]
async fn a_tighten_link_cannot_answer_tool_call() {
    let kernel = Next::new(|_| Box::pin(async { Ok(ToolOutput::text("kernel")) }));
    let report = Report::default();
    let result = run::<tool::Call>(
        &[link("answer", Tier::Tighten)],
        kernel,
        &cx(),
        call(),
        report.clone(),
    )
    .await
    .unwrap();
    assert_eq!(result, ToolOutput::text("kernel"));
    assert_eq!(report.take().len(), 1);
}
#[tokio::test]
async fn a_rewrite_allow_over_nexts_ask_is_skipped() {
    let kernel = Next::new(|call| {
        Box::pin(async {
            Ok(CheckResult {
                call,
                decision: Decision::Ask {
                    reason: Reason::Default,
                    scope: None,
                },
            })
        })
    });
    let report = Report::default();
    let result = run::<tool::Check>(
        &[link("loosen", Tier::Rewrite)],
        kernel,
        &cx(),
        call(),
        report.clone(),
    )
    .await
    .unwrap();
    assert!(matches!(result.decision, Decision::Ask { .. }));
    assert_eq!(report.take().len(), 1);
}

#[tokio::test]
async fn dynamic_links_are_read_again_and_receive_only_the_user_grant() {
    struct Source(std::sync::atomic::AtomicBool);
    struct Observer;
    #[async_trait::async_trait]
    impl Interceptor for Observer {
        fn id(&self) -> &str {
            "observer"
        }
        fn matcher(&self) -> LinkMatcher {
            LinkMatcher::default()
        }
    }
    #[async_trait::async_trait]
    impl InterceptorSource for Source {
        fn id(&self) -> &str {
            "source"
        }
        async fn interceptors(&self) -> Vec<ExternalInterceptor> {
            if self.0.load(std::sync::atomic::Ordering::Relaxed) {
                vec![ExternalInterceptor {
                    plugin: "outside".into(),
                    link: Arc::new(Observer),
                }]
            } else {
                vec![]
            }
        }
    }
    let source = Arc::new(Source(std::sync::atomic::AtomicBool::new(true)));
    let mut hooks = crate::turn::HookSet::default();
    hooks.interceptors.push(source.clone());
    assert_eq!(hooks.links().await[0].tier, Tier::Tighten);
    hooks.grants.insert(
        "outside".into(),
        PluginTrust {
            tier: Tier::Rewrite,
            uses: vec![],
        },
    );
    assert_eq!(hooks.links().await[0].tier, Tier::Rewrite);
    source.0.store(false, std::sync::atomic::Ordering::Relaxed);
    assert!(hooks.links().await.is_empty());
}

#[tokio::test(start_paused = true)]
async fn an_external_links_deadline_excludes_its_continuation() {
    let kernel = Next::new(|_| {
        Box::pin(async {
            tokio::time::sleep(HOOK_DEADLINE * 3).await;
            Ok(ToolOutput::text("kernel"))
        })
    });
    let report = Report::default();
    let result = run::<tool::Call>(
        &[link("observer", Tier::Tighten)],
        kernel,
        &cx(),
        call(),
        report.clone(),
    )
    .await
    .unwrap();
    assert_eq!(result, ToolOutput::text("kernel"));
    assert!(
        report.take().is_empty(),
        "waiting for next is not plugin time"
    );
}

#[tokio::test(start_paused = true)]
async fn draining_a_continuation_is_not_plugin_phase_time() {
    let kernel = Next::new(|_| Box::pin(async { Ok(ToolOutput::text("kernel")) }));
    let report = Report::default();
    let result = run::<tool::Call>(
        &[link("drain", Tier::Tighten)],
        kernel,
        &cx(),
        call(),
        report.clone(),
    )
    .await
    .unwrap();
    assert_eq!(result, ToolOutput::text("kernel"));
    assert!(report.take().is_empty());
}

#[tokio::test]
async fn a_json_deny_only_answer_never_runs_the_tool() {
    struct Deny;
    #[async_trait]
    impl Interceptor for Deny {
        fn id(&self) -> &str {
            "guard"
        }
        fn matcher(&self) -> LinkMatcher {
            LinkMatcher::default()
        }
        async fn chain(
            &self,
            _: &catalogue::Entry,
            _: &HookContext,
            _: serde_json::Value,
            _: Next<'_, serde_json::Value, serde_json::Value>,
        ) -> Result<serde_json::Value, KernelError> {
            Ok(serde_json::json!({"deny":"protected by guard"}))
        }
    }
    for tier in [Tier::Tighten, Tier::Rewrite, Tier::Full] {
        let count = AtomicUsize::new(0);
        let kernel = Next::new(|_| {
            Box::pin(async {
                count.fetch_add(1, Ordering::SeqCst);
                Ok(ToolOutput::text("must not run"))
            })
        });
        let result = run::<tool::Call>(
            &[Link {
                placement: bingo_sdk::plugin::Placement::Outer,
                external: Some("guard".into()),
                interceptor: Arc::new(Deny),
                tier,
            }],
            kernel,
            &cx(),
            call(),
            Report::default(),
        )
        .await;
        assert_eq!(
            count.load(Ordering::SeqCst),
            0,
            "denial must stop the tool at {tier:?}"
        );
        let error = result.unwrap_err();
        assert_eq!(error.code, ErrorCode::PermissionDenied);
        assert_eq!(error.message, "protected by guard");
    }
}
