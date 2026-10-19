//! `/permission [mode]`: what this session does when no rule decides.
//!
//! The mode a person names here is an answer, not a setting: it lives beside
//! the rules they accepted, in memory, for this session only — another session
//! and a sub-agent keep the configured mode, and no file is written (ADR-0008).
//!
//! It is not a second gate. Every mode may be named, `bypassPermissions`
//! included, because the ladder above the mode does not move: a deny rule, an
//! ask rule, a write into a sensitive directory and a tool's own confirmation
//! still stop the call in any mode.

use std::sync::Arc;

use async_trait::async_trait;
use bingo_sdk::{
    ArgSpec, Command, CommandContext, CommandOutcome, CommandSpec, ErrorCode, KernelError,
    SessionId, View,
};

use crate::PermissionsPolicy;
use crate::mode::Mode;

#[derive(Debug)]
pub struct PermissionCommand {
    policy: Arc<PermissionsPolicy>,
}

impl PermissionCommand {
    pub fn new(policy: Arc<PermissionsPolicy>) -> Self {
        Self { policy }
    }

    /// The mode this session runs in, and the five it could run in.
    fn listing(&self, session: &SessionId) -> String {
        let width = Mode::ALL
            .iter()
            .map(|mode| mode.as_str().len())
            .max()
            .unwrap_or(0);
        let modes = Mode::ALL
            .map(|mode| format!("  {:width$}  {}", mode.as_str(), mode.meaning()))
            .join("\n");
        format!(
            "permission mode: {}\n\n{modes}",
            self.policy.mode_for(session)
        )
    }
}

#[async_trait]
impl Command for PermissionCommand {
    fn now(&self, view: &bingo_sdk::command::SessionView<'_>) -> Option<String> {
        bingo_sdk::slot::modes(view.state).next()
    }

    fn spec(&self) -> CommandSpec {
        CommandSpec {
            opens: false,
            keys: vec![],
            hidden: false,
            name: "permission".into(),
            aliases: vec!["permissions".into()],
            hint: "what it may do without asking".into(),
            args: ArgSpec::Words {
                labels: Mode::ALL
                    .iter()
                    .filter(|mode| mode.label() != mode.as_str())
                    .map(|mode| (mode.as_str().to_string(), mode.label().to_string()))
                    .collect(),
                person: Mode::ALL.map(|mode| mode.as_str().to_string()).to_vec(),
                alone: Mode::ALL.map(|mode| mode.as_str().to_string()).to_vec(),
                values: Mode::ALL
                    .iter()
                    .map(|mode| mode.as_str().to_string())
                    .chain(["forget".into()])
                    .collect(),
                then: Some(Box::new(ArgSpec::Free {
                    hint: "<session rule>".into(),
                })),
            },
            // Reading and setting the mode touch nothing the turn is using; the
            // mode the next call is decided against is the one set last.
            instant: true,
            family: "session".into(),
        }
    }

    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let asked = args.trim();
        if asked.is_empty() {
            return Ok(CommandOutcome::View {
                view: View::text(self.listing(&cx.session)),
            });
        }
        if let Some(rule) = asked.strip_prefix("forget ") {
            let rule = rule.trim();
            if !self.policy.sessions.forget(&cx.session, rule) {
                return Err(KernelError::new(
                    ErrorCode::InvalidInput,
                    format!("no session rule: {rule}"),
                ));
            }
            return Ok(CommandOutcome::Applied {
                message: Some(format!("forgot: {rule} · asks again")),
            });
        }
        let mode: Mode = asked.parse().map_err(|unknown: crate::UnknownMode| {
            KernelError::new(ErrorCode::InvalidInput, unknown.to_string())
        })?;
        if !self.policy.sessions.change_mode(
            &cx.session,
            self.policy.mode,
            mode,
            bingo_sdk::render::asked(),
        ) {
            return Err(KernelError::new(
                ErrorCode::PermissionDenied,
                "only the person's own command may widen the permission mode",
            ));
        }
        // Read back rather than echo: the message says what the next call will
        // be decided against, in the words the footer uses (ADR-0095 §5).
        Ok(CommandOutcome::Applied {
            message: Some(format!(
                "permission mode: {}",
                self.policy.mode_for(&cx.session).label()
            )),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    use bingo_sdk::{Decision, PermissionPolicy, PolicyInput, Subject, ToolCall, ToolTraits};
    use serde_json::{Value, json};

    use crate::Settings;

    #[test]
    fn now_reads_the_supplied_sessions_mode_signal() {
        let summary = serde_json::from_value(json!({"id":"ses_view","cwd":"/work","driver":"model","createdAt":"2026-10-15T00:00:00Z","updatedAt":"2026-10-15T00:00:00Z","usage":bingo_sdk::Usage::default(),"busy":false,"asking":false})).unwrap();
        let mut state = bingo_sdk::SessionState::new(summary);
        state
            .signals
            .entry("permissions".into())
            .or_default()
            .insert(
                "mode".into(),
                bingo_sdk::slot::with(
                    json!({}),
                    [bingo_sdk::slot::Slot::Mode {
                        current: "plan".into(),
                        modes: vec![],
                    }],
                )
                .into(),
            );
        let policy = Arc::new(PermissionsPolicy::new(crate::Permissions::default(), None).unwrap());
        let command = PermissionCommand::new(policy);
        assert_eq!(
            command.now(&bingo_sdk::command::SessionView {
                state: &state,
                providers: &[],
                models: &[],
            }),
            Some("plan".into())
        );
    }

    /// A command context reads its session, its directory and a host;
    /// `/permission` asks the host nothing, so every answer here would be a bug.
    struct UnusedHost;

    impl UnusedHost {
        fn scripted(&self) -> bingo_sdk::testing::TestHost {
            let host = bingo_sdk::testing::TestHost::default();
            host.on_sessions(
                |_filter| async move { unreachable!("/permission reads no session list") },
            );
            host.on_open(|_selector, _who, _options| async move {
                unreachable!("/permission opens no session")
            });
            host.on_close(|_session, _reason| async move {
                unreachable!("/permission closes no session")
            });
            host.on_delete(
                |_session| async move { unreachable!("/permission deletes no session") },
            );
            host.on_deliver(|_to, _intent, _input, _delivery| async move {
                unreachable!("this double delivers nothing")
            });
            host.on_extend(|_session, _plugin, _kind, _payload| async move {
                unreachable!("this double extends nothing")
            });
            host.on_signal(|_session, _plugin, _kind, _payload| async move {
                unreachable!("this double signals nothing")
            });
            host.on_catalog(|_kind| async move { unreachable!("/permission reads no catalog") });
            host.on_gateway_events(|| unreachable!("/permission watches no gateway"));
            host
        }
    }

    fn context(session: &str) -> CommandContext {
        CommandContext {
            surface: None,
            cancel: bingo_sdk::CancellationToken::new(),
            session: SessionId::from_raw(session),
            cwd: PathBuf::from("/work/project"),
            host: UnusedHost.scripted().handle(),
        }
    }

    fn policy(settings: Value) -> Arc<PermissionsPolicy> {
        let settings: Settings = serde_json::from_value(settings).expect("a readable slice");
        Arc::new(
            PermissionsPolicy::new(settings.permissions, Some(PathBuf::from("/home/user")))
                .expect("readable rules"),
        )
    }

    /// What the gate would decide about an edit in the working tree, which is
    /// the call `acceptEdits` is about.
    async fn edit_in(policy: &PermissionsPolicy, session: &SessionId) -> Decision {
        let call = ToolCall {
            call_id: "call_1".into(),
            name: "Edit".into(),
            input: Value::Null,
        };
        let traits = ToolTraits {
            trusted: true,
            edit: true,
            ..ToolTraits::default()
        };
        let subjects = vec![Subject::Path {
            path: PathBuf::from("/work/project/src/main.rs"),
        }];
        policy
            .decide(PolicyInput {
                call: &call,
                traits: &traits,
                subjects: &subjects,
                confirm: None,
                session,
                cwd: Path::new("/work/project"),
            })
            .await
    }

    async fn typed(command: &PermissionCommand, args: &str, session: &str) -> CommandOutcome {
        bingo_sdk::render::under_input(true, command.run(args, &context(session)))
            .await
            .expect("a mode this crate knows")
    }

    #[tokio::test]
    async fn a_mode_chosen_in_one_session_decides_that_session_only() {
        let policy = policy(json!({}));
        let command = PermissionCommand::new(Arc::clone(&policy));
        let mine = SessionId::from_raw("ses_mine");
        let theirs = SessionId::from_raw("ses_theirs");

        assert!(
            matches!(edit_in(&policy, &mine).await, Decision::Ask { .. }),
            "an edit asks in the default mode"
        );

        let outcome = typed(&command, "acceptEdits", "ses_mine").await;
        assert_eq!(
            outcome,
            CommandOutcome::Applied {
                message: Some("permission mode: accept edits".into())
            }
        );

        assert!(matches!(
            edit_in(&policy, &mine).await,
            Decision::Allow { .. }
        ));
        assert!(
            matches!(edit_in(&policy, &theirs).await, Decision::Ask { .. }),
            "another session never inherits the mode"
        );
    }

    #[tokio::test]
    async fn the_session_s_mode_leaves_the_configured_one_alone() {
        let policy = policy(json!({ "permissions": { "defaultMode": "plan" } }));
        let command = PermissionCommand::new(Arc::clone(&policy));
        typed(&command, "acceptEdits", "ses_mine").await;

        assert_eq!(policy.mode, Mode::Plan, "the settings were written to");
        assert_eq!(
            policy.mode_for(&SessionId::from_raw("ses_fresh")),
            Mode::Plan,
            "a session that chose nothing runs configured"
        );
    }

    #[tokio::test]
    async fn no_argument_names_this_session_s_mode_and_the_five_to_choose_from() {
        let policy = policy(json!({}));
        let command = PermissionCommand::new(Arc::clone(&policy));
        typed(&command, "plan", "ses_mine").await;

        let CommandOutcome::View {
            view: View::Text { children, .. },
        } = typed(&command, "  ", "ses_mine").await
        else {
            panic!("a listing is a text view");
        };
        let text = bingo_sdk::view::text_children(&children);
        assert!(
            text.starts_with("permission mode: plan\n"),
            "the session's own mode comes first: {text}"
        );
        for mode in Mode::ALL {
            assert!(
                text.contains(mode.as_str()),
                "{mode} is not offered: {text}"
            );
            assert!(text.contains(mode.meaning()), "{mode} is unexplained");
        }
    }

    #[tokio::test]
    async fn a_mode_nobody_defined_is_refused_and_changes_nothing() {
        let policy = policy(json!({}));
        let command = PermissionCommand::new(Arc::clone(&policy));
        let session = SessionId::from_raw("ses_mine");

        let error = command
            .run("yolo", &context("ses_mine"))
            .await
            .expect_err("not a mode");
        assert_eq!(error.code, ErrorCode::InvalidInput);
        assert!(error.message.contains("bypassPermissions"), "{error}");
        assert_eq!(policy.mode_for(&session), Mode::Default);
    }

    #[tokio::test]
    async fn bypassing_the_gate_is_a_mode_like_any_other() {
        let policy = policy(json!({ "permissions": { "deny": ["Edit"] } }));
        let command = PermissionCommand::new(Arc::clone(&policy));
        let session = SessionId::from_raw("ses_mine");
        typed(&command, "bypassPermissions", "ses_mine").await;

        assert_eq!(policy.mode_for(&session), Mode::BypassPermissions);
        assert!(
            matches!(edit_in(&policy, &session).await, Decision::Deny { .. }),
            "a deny rule stands above every mode"
        );
    }

    /// The slot names the mode by the value `/permission` runs, so a chooser
    /// matches it against the spec's words; both carry the words to draw.
    #[test]
    fn every_mode_is_current_by_its_value_and_drawn_by_its_label() {
        for mode in Mode::ALL {
            let policy = policy(json!({ "permissions": { "defaultMode": mode } }));
            let session = SessionId::from_raw("ses_every_mode");
            let payload = policy.describe(&session);
            let slots: Vec<bingo_sdk::slot::Slot> =
                serde_json::from_value(payload["slots"].clone()).unwrap();
            let bingo_sdk::slot::Slot::Mode { current, modes } = &slots[0] else {
                panic!("mode")
            };
            assert_eq!(current, mode.as_str(), "the value, not the words");
            assert_eq!(bingo_sdk::slot::mode_label(current, modes), mode.label());
            let ArgSpec::Words { values, labels, .. } = PermissionCommand::new(policy).spec().args
            else {
                panic!("a mode is one of the words");
            };
            assert!(values.contains(current), "{current} is a word it offers");
            assert_eq!(
                labels.get(current).map_or(current.as_str(), String::as_str),
                mode.label()
            );
        }
    }

    #[test]
    fn the_spec_runs_now_and_takes_a_mode() {
        let spec = PermissionCommand::new(policy(json!({}))).spec();
        assert_eq!(spec.name, "permission");
        assert_eq!(spec.aliases, ["permissions"]);
        assert!(spec.instant, "reading a mode never waits for a turn");
        assert_eq!(spec.family, "session");
        let ArgSpec::Words { values, then, .. } = spec.args else {
            panic!("a mode is one of the words");
        };
        assert!(then.is_some());
        assert!(values.contains(&"forget".into()));
    }
}

#[cfg(test)]
mod forget_tests {
    use super::*;
    use bingo_sdk::{PermissionPolicy, testing::NoHost};
    #[tokio::test]
    async fn forget_removes_only_the_named_session_rule() {
        let policy = Arc::new(PermissionsPolicy::new(Default::default(), None).unwrap());
        let session = SessionId::from_raw("one");
        let other = SessionId::from_raw("other");
        for s in [&session, &other] {
            policy.sessions.install(s, "Bash(cargo:*)");
            policy.sessions.install(s, "Edit(src/**)");
        }
        let cx = CommandContext {
            surface: None,
            session: session.clone(),
            cwd: "/unused".into(),
            host: NoHost::handle(),
            cancel: Default::default(),
        };
        let command = PermissionCommand::new(policy.clone());
        assert!(matches!(
            command.run("forget Bash(cargo:*)", &cx).await.unwrap(),
            CommandOutcome::Applied { .. }
        ));
        assert_eq!(
            policy.describe(&session)["rules"],
            serde_json::json!(["Edit(src/**)"])
        );
        assert_eq!(policy.sessions.rules(&other).len(), 2);
        assert!(command.run("forget Bash(cargo:*)", &cx).await.is_err());
    }
}
