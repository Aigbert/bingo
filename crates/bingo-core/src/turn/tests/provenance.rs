use super::*;
use bingo_sdk::policy::RuleSource;

struct Answering(Decision);
#[async_trait]
impl PermissionPolicy for Answering {
    fn id(&self) -> &str {
        "fixture"
    }
    async fn decide(&self, _: PolicyInput<'_>) -> Decision {
        self.0.clone()
    }
}

struct AllowingHook;
#[async_trait]
impl Interceptor for AllowingHook {
    fn id(&self) -> &str {
        "review"
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
        let mut result = next.run(call).await?;
        result.decision = Decision::Allow {
            reason: Reason::Hook {
                hook: self.id().into(),
            },
        };
        Ok(result)
    }
}

fn policy_for(cfg: &mut TurnConfig, decision: Decision) {
    if matches!(
        decision,
        Decision::Allow {
            reason: Reason::Hook { .. }
        }
    ) {
        cfg.hooks.links.push(crate::chain::Link {
            plugin: "fixture".into(),
            placement: bingo_sdk::plugin::Placement::Outer,
            external: None,
            interceptor: Arc::new(AllowingHook),
            tier: Tier::Full,
        });
        cfg.policy = Arc::new(DefaultPolicy);
    } else {
        cfg.policy = Arc::new(Answering(decision));
    }
}

struct Edit;
#[async_trait]
impl Tool for Edit {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "Edit".into(),
            ..EchoTool { read_only: false }.spec()
        }
    }
    fn traits(&self, _: &serde_json::Value) -> ToolTraits {
        ToolTraits::edit()
    }
    fn subjects(&self, _: &serde_json::Value, cwd: &std::path::Path) -> Vec<Subject> {
        vec![Subject::Path {
            path: cwd.join("file.txt"),
        }]
    }
    async fn call(
        &self,
        input: serde_json::Value,
        cx: &ToolContext,
    ) -> Result<ToolOutput, ToolError> {
        EchoTool { read_only: false }.call(input, cx).await
    }
}
fn tool_for(decision: &Decision) -> Arc<dyn Tool> {
    match decision {
        Decision::Allow {
            reason: Reason::Mode { mode },
        } if mode == "acceptEdits" => Arc::new(Edit),
        Decision::Allow {
            reason: Reason::ReadOnly | Reason::Hook { .. },
        } => Arc::new(EchoTool { read_only: true }),
        _ => Arc::new(EchoTool { read_only: false }),
    }
}

async fn recorded(decision: Decision, expected: Option<Reason>) {
    let tool = tool_for(&decision);
    let provider = ScriptedProvider::new(vec![
        Script::Events(tool_call(
            &tool.spec().name,
            json!({"v":1,"file_path":"file.txt"}),
        )),
        Script::Events(text("done")),
    ]);
    let mut cfg = config(provider, vec![tool]);
    policy_for(&mut cfg, decision);
    let host = RecordingHost::new();
    host.answers.lock().unwrap().push_back(Answer::AllowOnce);
    let outcome = run(&cfg, &host, CancellationToken::new()).await;
    assert_eq!(outcome.status, TurnStatus::Completed);
    let mut running = 0;
    let mut completed = 0;
    for event in host.events() {
        let item = match event {
            Event::ItemUpdated { item } if item.status == ItemStatus::Running => {
                running += 1;
                item
            }
            Event::ItemCompleted { item } if matches!(item.body, ItemBody::ToolCall { .. }) => {
                completed += 1;
                item
            }
            _ => continue,
        };
        let ItemBody::ToolCall { allowed_by, .. } = item.body else {
            panic!("call")
        };
        assert_eq!(allowed_by.as_deref(), expected.as_ref());
    }
    assert_eq!((running, completed), (1, 1));
}
async fn allowed(reason: Reason) {
    recorded(
        Decision::Allow {
            reason: reason.clone(),
        },
        Some(reason),
    )
    .await;
}
#[tokio::test]
async fn accept_edits_records_the_mode() {
    allowed(Reason::Mode {
        mode: "acceptEdits".into(),
    })
    .await;
}
#[tokio::test]
async fn policy_modes_are_preserved() {
    allowed(Reason::Mode {
        mode: "fixture-mode".into(),
    })
    .await;
}
#[tokio::test]
async fn session_rules_keep_the_rule_and_source() {
    allowed(Reason::Rule {
        rule: "Echo".into(),
        source: Some(RuleSource::Session),
    })
    .await;
}
#[tokio::test]
async fn settings_rules_keep_the_rule_and_source() {
    allowed(Reason::Rule {
        rule: "Echo".into(),
        source: Some(RuleSource::Settings),
    })
    .await;
}
#[tokio::test]
async fn hook_allows_keep_the_hook() {
    allowed(Reason::Hook {
        hook: "review".into(),
    })
    .await;
}
#[tokio::test]
async fn read_only_records_no_auto_approval() {
    recorded(
        Decision::Allow {
            reason: Reason::ReadOnly,
        },
        None,
    )
    .await;
}
#[tokio::test]
async fn asking_records_no_auto_approval() {
    recorded(
        Decision::Ask {
            reason: Reason::Default,
            scope: None,
        },
        None,
    )
    .await;
}
