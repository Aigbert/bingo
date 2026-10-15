use crate::{REQUEST, Selections, error, rows::Row};
use bingo_sdk::*;
use serde_json::Value;
pub struct Handler(pub Selections);
pub fn selected(rows: &[Row], label: &str) -> usize {
    rows.iter()
        .position(|r| r.label == label && r.command.is_some())
        .or_else(|| rows.iter().position(|r| r.command.is_some()))
        .unwrap_or(0)
}
pub fn moved(rows: &[Row], label: &str, down: bool) -> String {
    let picks = rows
        .iter()
        .filter(|r| r.command.is_some())
        .collect::<Vec<_>>();
    let at = picks.iter().position(|r| r.label == label).unwrap_or(0);
    let at = if down {
        at.saturating_add(1).min(picks.len().saturating_sub(1))
    } else {
        at.saturating_sub(1)
    };
    picks.get(at).map(|r| r.label.clone()).unwrap_or_default()
}
#[async_trait::async_trait]
impl Command for Handler {
    fn spec(&self) -> CommandSpec {
        CommandSpec::ui(crate::ID)
    }
    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let args: Value = serde_json::from_str(args).map_err(error)?;
        if args["handler"] != REQUEST {
            return Ok(CommandOutcome::Handled { handled: false });
        }
        if args["event"] == "close" {
            crate::command::close(&cx.host, &cx.session, &self.0).await?;
            return Ok(CommandOutcome::Handled { handled: true });
        }
        let key = args["value"]["key"].as_str().unwrap_or_default();
        if args["event"] != "press" || !["up", "down", "enter"].contains(&key) {
            return Ok(CommandOutcome::Handled { handled: false });
        }
        let label = args["selected"].as_str().unwrap_or_default();
        let facts = (&crate::snapshot(&cx.host, &cx.session).await?.snapshot).into();
        let rows = crate::draw::read(&cx.host, &facts).await?;
        if !rows
            .iter()
            .any(|row| row.label == label && row.command.is_some())
        {
            return Ok(CommandOutcome::Handled { handled: true });
        }
        if key == "enter" {
            return enter(&rows, label, cx, &self.0).await;
        }
        self.0.lock().map_err(error)?.insert(
            cx.session.clone(),
            moved(&rows, label, key == "down").into(),
        );
        cx.host
            .invalidate(&cx.session, vec![REQUEST.into()])
            .await?;
        Ok(CommandOutcome::Handled { handled: true })
    }
}
async fn enter(
    rows: &[Row],
    label: &str,
    cx: &CommandContext,
    selections: &Selections,
) -> Result<CommandOutcome, KernelError> {
    let command = rows
        .iter()
        .find(|row| row.label == label)
        .and_then(|r| r.command.as_ref());
    if let Some(command) = command {
        crate::command::close(&cx.host, &cx.session, selections).await?;
        if matches!(command.as_str(), "rename" | "permission") {
            let text = format!("/{command} ");
            return Ok(CommandOutcome::Surface {
                action: Action {
                    name: "Composer".into(),
                    args: serde_json::json!({"text":text}),
                },
                fallback: Some(text),
            });
        }
        return crate::dispatch::run(command, cx).await;
    }
    Ok(CommandOutcome::Handled { handled: true })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_follows_labels_and_skips_information() {
        let rows = [
            "session",
            "spent",
            "permission",
            "allow Read",
            "plugins",
            "mcp",
        ]
        .map(|label| Row {
            label: label.into(),
            value: vec![],
            detail: None,
            command: (label != "spent").then(|| label.into()),
        });
        assert_eq!(moved(&rows, "session", true), "permission");
        assert_eq!(moved(&rows, "permission", false), "session");
        assert_eq!(moved(&rows, "mcp", true), "mcp");
        assert_eq!(moved(&rows, "session", false), "session");
        let without_rule = rows
            .iter()
            .filter(|r| r.label != "allow Read")
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            without_rule[selected(&without_rule, "plugins")].label,
            "plugins"
        );
    }
}
