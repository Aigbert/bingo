//! `/status`: what the session is right now, as one key-value view — the
//! facts the surface keeps off its screen until asked (design §7).

use std::sync::Weak;

use async_trait::async_trait;
use bingo_sdk::*;

use crate::host::Host;

pub(super) struct StatusCommand {
    pub(super) host: Weak<Host>,
}

#[async_trait]
impl Command for StatusCommand {
    fn spec(&self) -> CommandSpec {
        super::spec("status", "", ArgSpec::None, true)
    }

    async fn run(&self, _args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let state = super::host(&self.host)?.session_state(&cx.session).await?;
        Ok(CommandOutcome::View {
            view: View::KeyValue { rows: rows(&state) },
        })
    }
}

fn rows(state: &SessionState) -> Vec<(String, String)> {
    let s = &state.summary;
    let mut rows = vec![
        ("session".into(), s.id.to_string()),
        ("cwd".into(), s.cwd.clone()),
        (
            "provider".into(),
            s.provider.clone().unwrap_or_else(|| "?".into()),
        ),
        (
            "model".into(),
            s.model.clone().unwrap_or_else(|| "?".into()),
        ),
        ("mode".into(), mode(state)),
        ("context".into(), context(state.context)),
        (
            "tokens".into(),
            format!(
                "{} in · {} out",
                s.usage.input_tokens, s.usage.output_tokens
            ),
        ),
    ];
    if let Some(parent) = &s.parent {
        rows.push(("parent".into(), parent.session.to_string()));
    }
    rows
}

fn mode(state: &SessionState) -> String {
    bingo_sdk::slot::placed(state)
        .into_iter()
        .find_map(|placed| match placed.slot {
            bingo_sdk::slot::Slot::Mode { current, .. } => Some(current),
            _ => None,
        })
        .unwrap_or_else(|| "default".into())
}

fn context(context: Option<ContextUsage>) -> String {
    match context {
        Some(c) => format!(
            "{} of {} tokens ({} %) · compacts at {}",
            c.used,
            c.window,
            c.percent(),
            c.trigger
        ),
        None => "not measured yet".into(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn rough_edges_status_groups_digits() {
        assert_eq!(
            context(Some(ContextUsage {
                used: 13_812,
                window: 191_808,
                trigger: 172_627
            })),
            "13,812 of 191,808 tokens (7 %) · compacts at 172,627"
        );
        let mut state = SessionState::new(crate::test_support::summary("ses_status"));
        state.summary.usage.input_tokens = 54_811;
        state.summary.usage.output_tokens = 139;
        assert!(rows(&state).contains(&("tokens".into(), "54,811 in · 139 out".into())));
    }

    use super::*;

    #[test]
    fn the_mode_is_the_policy_s_or_default() {
        let mut state = SessionState::new(crate::test_support::summary("ses_mode"));
        assert_eq!(mode(&state), "default");
        state.config.plugins.insert(
            "bingo.permissions".into(),
            bingo_sdk::slot::with(
                serde_json::json!({}),
                [bingo_sdk::slot::Slot::Mode {
                    current: "acceptEdits".into(),
                    modes: vec![],
                }],
            ),
        );
        assert_eq!(mode(&state), "acceptEdits");
    }

    #[test]
    fn the_context_row_reads_as_a_fraction() {
        assert_eq!(context(None), "not measured yet");
        let usage = ContextUsage {
            used: 42_000,
            window: 100_000,
            trigger: 80_000,
        };
        assert_eq!(
            context(Some(usage)),
            "42000 of 100000 tokens (42 %) · compacts at 80000"
        );
    }
}
