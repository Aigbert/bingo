use crate::{ID, REQUEST, Shared};
use bingo_sdk::*;
use bingo_services::schedule::Row;
use serde_json::{Value, json};
pub struct Handler(pub Shared);
#[async_trait::async_trait]
impl Command for Handler {
    fn spec(&self) -> CommandSpec {
        CommandSpec::ui(ID)
    }
    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let args: Value = serde_json::from_str(args).map_err(crate::error)?;
        if !render::vouched() || args["handler"] != REQUEST {
            return Ok(handled());
        }
        let press = sheet::pressed(&args);
        let key = press
            .as_ref()
            .and_then(sheet::Press::key)
            .unwrap_or_default();
        let picked = press.as_ref().and_then(sheet::Press::picked);
        let key = if picked.is_some() { "enter" } else { key };
        if args["event"] == "close" || matches!(key, "esc" | "ctrl+c") {
            crate::panel::close(cx, &self.0).await?;
            return Ok(handled());
        }
        let listing = crate::listing(&cx.host).await?;
        let shown: Option<Row> = if let Some(id) = picked {
            let rows: Vec<Row> =
                serde_json::from_value(args["rows"].clone()).map_err(crate::error)?;
            rows.into_iter().find(|row| row.id == id)
        } else {
            serde_json::from_value(args["row"].clone()).map_err(crate::error)?
        };
        let row = {
            let mut shared = self.0.lock().map_err(crate::error)?;
            let Some(cursor) = shared.get_mut(&cx.session) else {
                return Ok(handled());
            };
            let rows = listing.entries.clone();
            if let Some(id) = picked {
                cursor.selected = Some(id.into());
            }
            let at = crate::rows::selected(&rows, cursor.selected.as_deref());
            if matches!(key, "up" | "down") {
                let picks = rows
                    .iter()
                    .map(|row| pick::Row::new(&row.id, pick::text("", "fg")))
                    .collect::<Vec<_>>();
                cursor.selected = pick::moved(&picks, cursor.selected.as_deref(), key == "down");
                None
            } else {
                listing
                    .entries
                    .get(at)
                    .filter(|row| Some(*row) == shown.as_ref())
                    .cloned()
            }
        };
        let result = if let Some(row) = row {
            self.act(&row, key, cx).await?
        } else {
            handled()
        };
        cx.host
            .invalidate(&cx.session, vec![REQUEST.into()])
            .await?;
        Ok(result)
    }
}
impl Handler {
    async fn act(
        &self,
        row: &Row,
        key: &str,
        cx: &CommandContext,
    ) -> Result<CommandOutcome, KernelError> {
        match key {
            "space" | " " => {
                if row.id.is_empty()
                    || !row
                        .id
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
                {
                    return Ok(CommandOutcome::Applied {
                        message: Some("schedule id cannot be carried on a command line".into()),
                    });
                }
                Ok(CommandOutcome::Surface {
                    action: Action {
                        name: "surface:pane".into(),
                        args: json!({"plugin":ID,"id":REQUEST,"value":{"run":format!("/schedule {} {}", if row.enabled { "pause" } else { "resume" }, row.id)}}),
                    },
                    fallback: None,
                })
            }
            "enter" => {
                if bingo_services::schedule::visible(&row.text) != row.text {
                    return Ok(CommandOutcome::Applied {
                        message: Some(
                            "schedule contains hidden characters; edit its file before running"
                                .into(),
                        ),
                    });
                }
                crate::panel::close(cx, &self.0).await?;
                Ok(CommandOutcome::Surface {
                    action: Action {
                        name: "SubmitAtHome".into(),
                        args: json!({"text":row.text}),
                    },
                    fallback: None,
                })
            }
            _ => Ok(handled()),
        }
    }
}
fn handled() -> CommandOutcome {
    CommandOutcome::Handled { handled: true }
}
