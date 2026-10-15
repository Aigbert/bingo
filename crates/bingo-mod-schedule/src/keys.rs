use crate::{Confirmation, Cursor, ID, REQUEST, Shared};
use bingo_sdk::*;
use bingo_services::schedule::Row;
use serde_json::{Value, json};
pub struct Handler(pub Shared);
fn press(
    cursor: &mut Cursor,
    rows: &[Row],
    shown: Option<&Row>,
    confirming: Option<&Confirmation>,
    key: &str,
) -> Value {
    if key == "esc" && cursor.confirming.take().is_some() {
        return json!({});
    }
    if matches!(key, "esc" | "ctrl+c") {
        return json!({"close":true});
    }
    let at = rows
        .iter()
        .position(|r| r.id == cursor.selected)
        .unwrap_or(0);
    if matches!(key, "up" | "down") {
        let at = at
            .saturating_add_signed(if key == "up" { -1 } else { 1 })
            .min(rows.len().saturating_sub(1));
        cursor.selected = rows.get(at).map(|r| r.id.clone()).unwrap_or_default();
        cursor.confirming = None;
        return json!({});
    }
    let Some(row) = rows.get(at).filter(|row| Some(*row) == shown) else {
        cursor.confirming = None;
        return json!({"message":"the list changed; look again"});
    };
    if cursor.confirming.is_some() {
        if key == "enter"
            && confirming.is_some_and(|shown| shown.row() == row)
            && cursor.confirming.as_ref() == confirming
        {
            let Some(confirmation) = cursor.confirming.take() else {
                return json!({});
            };
            return json!({"run":format!("/schedule {} {}", confirmation.verb(), row.id)});
        }
        return json!({});
    }
    operation(cursor, row, key)
}
fn operation(cursor: &mut Cursor, row: &Row, key: &str) -> Value {
    match key {
        "space" | " " => {
            json!({"run":format!("/schedule {} {}", if row.enabled { "pause" } else { "resume" }, row.id)})
        }
        "r" => json!({"run":format!("/schedule run {}", row.id)}),
        "d" => {
            cursor.confirming = Some(Confirmation::Forget(row.clone()));
            json!({})
        }
        "c" if row.by != "you" => {
            cursor.confirming = Some(Confirmation::Confirm(row.clone()));
            json!({})
        }
        "right" => match &row.session {
            Some(session) => json!({"view":session}),
            None => json!({"message":"has not fired yet"}),
        },
        _ => json!({}),
    }
}
#[async_trait::async_trait]
impl Command for Handler {
    fn spec(&self) -> CommandSpec {
        CommandSpec::ui(ID)
    }
    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let args: Value = serde_json::from_str(args).map_err(crate::error)?;
        if !render::vouched() || args["handler"] != REQUEST {
            return Ok(CommandOutcome::Handled { handled: true });
        }
        if args["event"] == "close" {
            crate::panel::close(cx, &self.0).await?;
            return Ok(CommandOutcome::Handled { handled: true });
        }
        let shown: Option<Row> =
            serde_json::from_value(args["row"].clone()).map_err(crate::error)?;
        let confirming: Option<Confirmation> =
            serde_json::from_value(args["confirming"].clone()).map_err(crate::error)?;
        let listing = crate::listing(&cx.host).await?;
        let operation = {
            let mut shared = self.0.lock().map_err(crate::error)?;
            let Some(cursor) = shared.get_mut(&cx.session) else {
                return Ok(CommandOutcome::Handled { handled: true });
            };
            press(
                cursor,
                &listing.entries,
                shown.as_ref(),
                confirming.as_ref(),
                args["value"]["key"].as_str().unwrap_or_default(),
            )
        };
        if operation["close"] == true || operation.get("view").is_some() {
            crate::panel::close(cx, &self.0).await?;
        }
        cx.host
            .invalidate(&cx.session, vec![REQUEST.into()])
            .await?;
        Ok(CommandOutcome::Surface {
            action: Action {
                name: "surface:pane".into(),
                args: json!({"plugin":ID,"id":REQUEST,"value":operation}),
            },
            fallback: None,
        })
    }
}
#[cfg(test)]
#[path = "keys_tests.rs"]
mod tests;
