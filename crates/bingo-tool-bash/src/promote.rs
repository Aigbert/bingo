//! Promotion: the flag a running foreground command listens for, and the
//! command that flips it.
//!
//! Backgrounding a command thirty seconds in is a judgement a person makes,
//! not a flag guessed up front (ADR-0018 §6), so the decision has to reach a
//! call that is already in flight. Every foreground `Bash` call leaves a token
//! here under its call id while it runs; `bash.promote` flips the one it
//! names, the call's wait ends, and the same process goes to the job table.
//! The listener takes its entry away when the call is over, so a call id is
//! never promotable twice.
//!
//! The command is `instant`, because a turn is running by definition when
//! there is anything to promote, and it takes the call id as its argument —
//! which is what an `Input::Action` from a surface carries (ADR-0008 §1).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use bingo_sdk::{
    ArgSpec, CancellationToken, Command, CommandContext, CommandOutcome, CommandSpec, KernelError,
};

/// The name a surface fires to background the command it is watching. A
/// surface cannot import this crate (ADR-0001), so the string is the contract.
pub const ACTION: &str = "bash.promote";

/// The foreground calls that can still be taken into the background.
#[derive(Debug, Default)]
pub struct Promotions {
    open: Mutex<BTreeMap<String, Foreground>>,
}

#[derive(Debug, Clone)]
struct Foreground {
    token: CancellationToken,
    session: bingo_sdk::SessionId,
    item: bingo_sdk::ItemId,
}

impl Promotions {
    /// Listen for a promotion of `call` until the guard is dropped.
    pub fn listen(
        self: &Arc<Self>,
        call: &str,
        session: &bingo_sdk::SessionId,
        item: &bingo_sdk::ItemId,
    ) -> Listener {
        let token = CancellationToken::new();
        self.locked().insert(
            call.to_string(),
            Foreground {
                token: token.clone(),
                session: session.clone(),
                item: item.clone(),
            },
        );
        Listener {
            promotions: self.clone(),
            call: call.to_string(),
            token,
        }
    }

    /// Flip the flag for `call`; `false` when no such call is running.
    pub fn promote(&self, call: &str) -> bool {
        let Some(token) = self.locked().get(call.trim()).cloned() else {
            return false;
        };
        token.token.cancel();
        true
    }

    /// The calls a surface could name, for the message when it named none.
    pub fn open(&self) -> Vec<String> {
        self.locked().keys().cloned().collect()
    }

    pub fn rows(&self, session: &bingo_sdk::SessionId) -> Vec<bingo_sdk::slot::Slot> {
        self.locked()
            .iter()
            .filter(|(_, call)| &call.session == session && !call.token.is_cancelled())
            .map(|(id, call)| bingo_sdk::slot::Slot::Row {
                caption: None,
                item: call.item.clone(),
                mark: None,
                title: None,
                badge: None,
                body: None,
                actions: vec![bingo_sdk::ActionItem {
                    label: "background".into(),
                    action: bingo_sdk::Action {
                        name: ACTION.into(),
                        args: serde_json::Value::String(id.clone()),
                    },
                    key: Some('b'),
                }],
            })
            .collect()
    }

    fn locked(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, Foreground>> {
        self.open.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// One call's entry, taken away when the call ends however it ends.
#[derive(Debug)]
pub struct Listener {
    promotions: Arc<Promotions>,
    call: String,
    token: CancellationToken,
}

impl Listener {
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.promotions.locked().remove(&self.call);
    }
}

/// What a surface fires, and what a person may type.
pub struct PromoteCommand {
    promotions: Arc<Promotions>,
}

impl PromoteCommand {
    pub fn new(promotions: Arc<Promotions>) -> Self {
        Self { promotions }
    }
}

#[async_trait]
impl Command for PromoteCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            hidden: false,
            name: ACTION.into(),
            aliases: Vec::new(),
            hint: "background the running shell command".into(),
            args: ArgSpec::Free {
                hint: "the call id".into(),
            },
            // There is only anything to promote while a turn is running.
            instant: true,
            family: "shell".into(),
        }
    }

    async fn run(&self, args: &str, _cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let message = match self.promotions.promote(args) {
            true => "backgrounding it".into(),
            false => nothing(&self.promotions.open(), args),
        };
        Ok(CommandOutcome::Applied {
            message: Some(message),
        })
    }
}

/// Why nothing was promoted: no command is running at all, or not that one.
fn nothing(open: &[String], asked: &str) -> String {
    match open {
        [] => "no shell command is running".into(),
        _ if asked.trim().is_empty() => "say which call to background".into(),
        _ => format!("no shell command is running as `{}`", asked.trim()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_listening_call_is_promoted_by_name() {
        let promotions = Arc::new(Promotions::default());
        let listener = promotions.listen(
            "call_1",
            &bingo_sdk::SessionId::from_raw("ses_1"),
            &bingo_sdk::ItemId::from_raw("call_1"),
        );
        assert_eq!(promotions.open(), ["call_1"]);
        let rows = promotions.rows(&bingo_sdk::SessionId::from_raw("ses_1"));
        assert_eq!(
            rows,
            [bingo_sdk::slot::Slot::Row {
                item: bingo_sdk::ItemId::from_raw("call_1"),
                mark: None,
                title: None,
                badge: None,
                caption: None,
                body: None,
                actions: vec![bingo_sdk::ActionItem {
                    label: "background".into(),
                    action: bingo_sdk::Action {
                        name: "bash.promote".into(),
                        args: serde_json::json!("call_1"),
                    },
                    key: Some('b'),
                }],
            }]
        );
        assert!(
            promotions
                .rows(&bingo_sdk::SessionId::from_raw("other"))
                .is_empty()
        );
        assert!(!listener.token().is_cancelled());
        assert!(promotions.promote("call_1"));
        assert!(listener.token().is_cancelled());
    }

    #[test]
    fn a_call_nobody_is_running_is_not_promoted() {
        let promotions = Arc::new(Promotions::default());
        assert!(!promotions.promote("call_1"));
        assert!(promotions.open().is_empty());
    }

    #[test]
    fn a_call_that_has_ended_can_no_longer_be_promoted() {
        let promotions = Arc::new(Promotions::default());
        drop(promotions.listen(
            "call_1",
            &bingo_sdk::SessionId::from_raw("ses_1"),
            &bingo_sdk::ItemId::from_raw("call_1"),
        ));
        assert!(!promotions.promote("call_1"));
        assert!(promotions.open().is_empty());
    }

    #[test]
    fn two_calls_are_promoted_apart() {
        let promotions = Arc::new(Promotions::default());
        let one = promotions.listen(
            "call_1",
            &bingo_sdk::SessionId::from_raw("ses_1"),
            &bingo_sdk::ItemId::from_raw("call_1"),
        );
        let two = promotions.listen(
            "call_2",
            &bingo_sdk::SessionId::from_raw("ses_1"),
            &bingo_sdk::ItemId::from_raw("call_2"),
        );
        assert!(promotions.promote(" call_2 "), "a name is trimmed");
        assert!(!one.token().is_cancelled());
        assert!(two.token().is_cancelled());
    }

    /// What the surface fires, and what a person may type.
    async fn fired(promotions: &Arc<Promotions>, args: &str) -> String {
        let cx = crate::tests::command_context(std::env::temp_dir());
        let outcome = PromoteCommand::new(promotions.clone())
            .run(args, &cx)
            .await
            .expect("the command answered");
        let CommandOutcome::Applied { message } = outcome else {
            panic!("promoting applies at once, got {outcome:?}");
        };
        message.expect("a promotion always says what happened")
    }

    #[tokio::test]
    async fn the_command_flips_the_call_it_names() {
        let promotions = Arc::new(Promotions::default());
        let listener = promotions.listen(
            "call_1",
            &bingo_sdk::SessionId::from_raw("ses_1"),
            &bingo_sdk::ItemId::from_raw("call_1"),
        );
        assert_eq!(fired(&promotions, "call_1").await, "backgrounding it");
        assert!(listener.token().is_cancelled());
    }

    #[tokio::test]
    async fn the_command_says_why_it_promoted_nothing() {
        let promotions = Arc::new(Promotions::default());
        assert_eq!(
            fired(&promotions, "call_1").await,
            "no shell command is running"
        );
        let _listening = promotions.listen(
            "call_1",
            &bingo_sdk::SessionId::from_raw("ses_1"),
            &bingo_sdk::ItemId::from_raw("call_1"),
        );
        assert_eq!(
            fired(&promotions, "call_2").await,
            "no shell command is running as `call_2`"
        );
        assert_eq!(
            fired(&promotions, "  ").await,
            "say which call to background"
        );
    }

    #[test]
    fn the_command_runs_during_a_turn_and_takes_the_call_id() {
        let spec = PromoteCommand::new(Arc::new(Promotions::default())).spec();
        assert_eq!(spec.name, ACTION);
        assert!(spec.instant, "there is only anything to promote mid-turn");
        assert_eq!(spec.family, "shell");
        assert!(matches!(spec.args, ArgSpec::Free { .. }));
    }
}
