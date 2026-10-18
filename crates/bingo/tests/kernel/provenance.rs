use async_trait::async_trait;
use bingo_core::gate::{GateInput, gate_call};
use bingo_permissions::{Mode, Permissions, PermissionsPolicy};
use bingo_sdk::*;
use serde_json::json;
use std::sync::Arc;

struct Read(bool);
#[async_trait]
impl Tool for Read {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "Read".into(),
            description: "fixture".into(),
            input_schema: json!({}),
            look: None,
            meta: Default::default(),
        }
    }
    fn traits(&self, _: &serde_json::Value) -> ToolTraits {
        ToolTraits {
            read_only: self.0,
            ..ToolTraits::read_only()
        }
    }
    async fn call(&self, _: serde_json::Value, _: &ToolContext) -> Result<ToolOutput, ToolError> {
        unreachable!("gate only")
    }
}

async fn probe(
    settings: Permissions,
    session_rule: bool,
    read_only: bool,
) -> bingo_core::gate::Gated {
    let policy = PermissionsPolicy::new(settings, None).unwrap();
    let session = SessionId::from_raw("s");
    let cwd = std::path::Path::new("/work");
    let call = ToolCall {
        call_id: "c".into(),
        name: "Read".into(),
        input: json!({}),
    };
    let traits = ToolTraits::read_only();
    if session_rule {
        policy
            .on_verdict(
                PolicyInput {
                    call: &call,
                    traits: &traits,
                    subjects: &[],
                    confirm: None,
                    session: &session,
                    cwd,
                },
                &Verdict::Allow {
                    scope: Some("Read".into()),
                },
            )
            .await;
    }
    if session_rule {
        let writable = ToolTraits {
            read_only: false,
            ..traits
        };
        assert_eq!(
            policy
                .decide(PolicyInput {
                    call: &call,
                    traits: &writable,
                    subjects: &[],
                    confirm: None,
                    session: &session,
                    cwd,
                })
                .await,
            Decision::Allow {
                reason: Reason::Rule {
                    rule: "Read".into(),
                    source: Some(bingo_sdk::policy::RuleSource::Session),
                }
            }
        );
        assert_eq!(
            policy
                .decide(PolicyInput {
                    call: &call,
                    traits: &ToolTraits::read_only(),
                    subjects: &[],
                    confirm: None,
                    session: &session,
                    cwd,
                })
                .await,
            Decision::Allow {
                reason: Reason::ReadOnly
            }
        );
    }
    let cx = HookContext {
        item: None,
        session: session.clone(),
        turn: None,
        cwd: cwd.into(),
        provider: None,
        model: None,
        host: bingo_sdk::testing::NoHost::handle(),
    };
    let gated = gate_call(
        GateInput {
            session: &session,
            cwd,
            item: &ItemId::from_raw("i"),
            call,
            tool: Some(Arc::new(Read(read_only))),
            policy: &policy,
            links: &[],
            hooks: &[],
            hook_cx: &cx,
        },
        &Nobody,
    )
    .await;
    assert!(matches!(gated.gate, bingo_core::executor::Gate::Allowed));
    gated
}

async fn read(settings: Permissions, session_rule: bool) {
    assert_eq!(probe(settings, session_rule, true).await.allowed_by, None);
}

#[tokio::test]
async fn a_read_in_bypass_records_no_allow_reason() {
    read(
        Permissions {
            default_mode: Some(Mode::BypassPermissions),
            ..Default::default()
        },
        false,
    )
    .await;
}
#[tokio::test]
async fn a_read_matching_settings_records_no_allow_reason() {
    read(
        Permissions {
            allow: vec!["Read".into()],
            ..Default::default()
        },
        false,
    )
    .await;
}
#[tokio::test]
async fn a_read_matching_session_records_no_allow_reason() {
    let writable = probe(Permissions::default(), true, false).await;
    assert_eq!(
        writable.allowed_by,
        Some(Reason::Rule {
            rule: "Read".into(),
            source: Some(bingo_sdk::policy::RuleSource::Session),
        })
    );
    read(Permissions::default(), true).await;
}

struct Nobody;
#[async_trait]
impl bingo_core::gate::GatePrompter for Nobody {
    async fn ask(
        &self,
        _: InteractionKind,
        _: Vec<AnswerSpec>,
    ) -> Result<bingo_core::gate::Answered, KernelError> {
        panic!("a read should not ask")
    }
}

#[tokio::test]
async fn a_non_read_in_bypass_keeps_the_mode_reason() {
    let gated = probe(
        Permissions {
            default_mode: Some(Mode::BypassPermissions),
            ..Default::default()
        },
        false,
        false,
    )
    .await;
    assert_eq!(
        gated.allowed_by,
        Some(Reason::Mode {
            mode: Mode::BypassPermissions.to_string()
        })
    );
}
