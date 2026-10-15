use crate::{
    Shared,
    rows::Target,
    state::{Cursor, Sheet},
};
use bingo_sdk::{component::SessionsProps, *};
use serde_json::{Value, json};
pub(crate) struct Open;
#[async_trait::async_trait]
impl Command for Open {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            name: "sessions".into(),
            aliases: vec![],
            hint: "find every session".into(),
            args: ArgSpec::None,
            instant: true,
            family: "terminal".into(),
            hidden: false,
            keys: vec![],
        }
    }
    async fn run(&self, _: &str, _: &CommandContext) -> Result<CommandOutcome, KernelError> {
        Ok(CommandOutcome::Surface {
            action: Action {
                name: "Sessions".into(),
                args: json!({}),
            },
            fallback: Some("/sessions is terminal only.".into()),
        })
    }
}
pub(crate) struct Handler(pub Shared);
#[derive(Debug, PartialEq)]
enum Operation {
    Surface(Value),
    Preview(SessionId),
    Rename(SessionId, String),
    Delete(SessionId),
    Confirm(Vec<crate::verbs::Deletion>),
    Fork(SessionId),
    None,
    Unhandled,
}
#[async_trait::async_trait]
impl Command for Handler {
    fn spec(&self) -> CommandSpec {
        CommandSpec::ui("sessions")
    }
    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let args: Value = serde_json::from_str(args).map_err(crate::error)?;
        if args["event"] == "activate" {
            return self.row(&args, cx).await;
        }
        if args["event"] != "press" {
            return Ok(CommandOutcome::Handled { handled: false });
        }
        let request = args["request"]
            .as_str()
            .or_else(|| args["handler"].as_str())
            .unwrap_or_default();
        let key = args["value"]["key"].as_str().unwrap_or_default();
        let props: SessionsProps =
            serde_json::from_value(args["props"].clone()).map_err(crate::error)?;
        let operation = self.press(request, key, &props)?;
        let outcome = self.act(operation, request, &props, cx).await;
        cx.host
            .invalidate(&cx.session, vec![request.into()])
            .await?;
        outcome
    }
}
impl Handler {
    async fn row(&self, args: &Value, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let request = args["request"].as_str().unwrap_or_default();
        let owner: SessionId =
            serde_json::from_value(args["session"].clone()).map_err(crate::error)?;
        let target: Target =
            serde_json::from_value(args["target"].clone()).map_err(crate::error)?;
        let operation = {
            let mut state = self.0.lock().map_err(crate::error)?;
            let Some(cursor) = state.cursors.get_mut(&(owner.clone(), request.into())) else {
                return Ok(CommandOutcome::Handled { handled: false });
            };
            if !matches!(cursor.sheet, Sheet::None) {
                return Ok(CommandOutcome::Handled { handled: true });
            }
            // M174 rebinds this keyed control against the current drawing before dispatch.
            cursor.selected = Some(target.clone());
            let operation = activate(cursor, Some(target));
            if matches!(&operation, Operation::Surface(value) if value.get("resume").is_some()) {
                state.cursors.remove(&(owner.clone(), request.into()));
            }
            operation
        };
        cx.host.invalidate(&owner, vec![request.into()]).await?;
        if let Operation::Surface(value) = operation {
            let mut action: Action =
                serde_json::from_value(args["action"].clone()).map_err(crate::error)?;
            action.args["value"] = value;
            Ok(CommandOutcome::Surface {
                action,
                fallback: None,
            })
        } else {
            Ok(CommandOutcome::Handled { handled: true })
        }
    }
    fn press(
        &self,
        request: &str,
        key: &str,
        props: &SessionsProps,
    ) -> Result<Operation, KernelError> {
        let mut state = self.0.lock().map_err(crate::error)?;
        let cursor = state
            .cursors
            .entry((props.session.clone(), request.into()))
            .or_default();
        cursor.sync(&props.query);
        let rows = crate::rows::listed(props, cursor);
        let target = cursor
            .selected
            .as_ref()
            .and_then(|selected| rows.iter().find(|r| &r.target == selected))
            .map(|r| r.target.clone());
        let operation = if let Sheet::Rename { id, .. } = &cursor.sheet {
            if matches!(key, "esc" | "ctrl+c")
                || matches!(&target, Some(Target::Session(target)) if target == id)
            {
                edit(cursor, key)
            } else {
                Operation::None
            }
        } else if matches!(key, "up" | "down") {
            navigate(cursor, &rows, key);
            Operation::None
        } else {
            operation(cursor, target, rows.len(), key, props)
        };
        if matches!(&operation, Operation::Surface(v) if v["close"] == true || v.get("resume").is_some())
        {
            state
                .cursors
                .remove(&(props.session.clone(), request.into()));
        }
        Ok(operation)
    }
    async fn act(
        &self,
        operation: Operation,
        request: &str,
        props: &SessionsProps,
        cx: &CommandContext,
    ) -> Result<CommandOutcome, KernelError> {
        let value = match operation {
            Operation::Surface(value) => value,
            Operation::None => return Ok(CommandOutcome::Handled { handled: true }),
            Operation::Unhandled => return Ok(CommandOutcome::Handled { handled: false }),
            Operation::Preview(id) => {
                crate::preview::start(
                    self.0.clone(),
                    cx.host.clone(),
                    props.session.clone(),
                    request.into(),
                    id,
                );
                return Ok(CommandOutcome::Handled { handled: true });
            }
            Operation::Rename(id, title) => {
                crate::verbs::rename(&cx.host, &id, &title).await?;
                if let Some(cursor) = self
                    .0
                    .lock()
                    .map_err(crate::error)?
                    .cursors
                    .get_mut(&(props.session.clone(), request.into()))
                {
                    cursor.sheet = Sheet::None;
                }
                json!({"refresh":true})
            }
            Operation::Fork(id) => json!({"resume":cx.host.fork(&id, None).await?}),
            Operation::Delete(id) => {
                let doomed = crate::verbs::descendants(&cx.host, &id, &props.session).await?;
                if let Some(cursor) = self
                    .0
                    .lock()
                    .map_err(crate::error)?
                    .cursors
                    .get_mut(&(props.session.clone(), request.into()))
                {
                    cursor.sheet = Sheet::Delete(doomed);
                }
                return Ok(CommandOutcome::Handled { handled: true });
            }
            Operation::Confirm(doomed) => {
                crate::verbs::delete(&cx.host, &doomed).await?;
                json!({"refresh":true})
            }
        };
        let mut action = props.action.clone();
        action.args["value"] = value;
        Ok(CommandOutcome::Surface {
            action,
            fallback: None,
        })
    }
}
fn operation(
    cursor: &mut Cursor,
    target: Option<Target>,
    _count: usize,
    key: &str,
    props: &SessionsProps,
) -> Operation {
    if key == "ctrl+c" {
        return Operation::Surface(json!({"close":true}));
    }
    if key == "esc" {
        return escape(cursor, props);
    }
    if let Sheet::Delete(doomed) = &cursor.sheet {
        return if key == "enter" {
            let doomed = doomed.clone();
            cursor.sheet = Sheet::None;
            Operation::Confirm(doomed)
        } else {
            Operation::None
        };
    }
    match key {
        "tab" => {
            cursor.selected = None;
            cursor.sheet = Sheet::None;
            return Operation::Surface(
                json!({"scope":if props.scope == component::SessionsScope::Here { "all" } else { "here" }}),
            );
        }
        "/" if props.query.is_empty() => {
            cursor.searching = true;
        }
        "enter" => return activate(cursor, target),
        "right" => return preview(cursor, target),
        "r" | "d" | "f" if props.query.is_empty() && !cursor.searching => {
            if let Some(Target::Session(id)) = target {
                return match key {
                    "r" => {
                        let text = props
                            .sessions
                            .iter()
                            .find(|s| s.id == id)
                            .and_then(|s| s.title.clone())
                            .unwrap_or_default();
                        cursor.sheet = Sheet::Rename { id, text };
                        Operation::None
                    }
                    "d" => Operation::Delete(id),
                    _ => Operation::Fork(id),
                };
            }
        }
        _ => return Operation::Unhandled,
    }
    Operation::None
}
fn navigate(cursor: &mut Cursor, rows: &[crate::rows::Row], key: &str) {
    let Some(at) = rows
        .iter()
        .position(|row| Some(&row.target) == cursor.selected.as_ref())
    else {
        return;
    };
    let next = if key == "up" {
        at.saturating_sub(1)
    } else {
        (at + 1).min(rows.len().saturating_sub(1))
    };
    cursor.selected = rows.get(next).map(|row| row.target.clone());
    cursor.sheet = Sheet::None;
}

fn edit(cursor: &mut Cursor, key: &str) -> Operation {
    if key == "esc" {
        cursor.sheet = Sheet::None;
        return Operation::None;
    }
    if key == "ctrl+c" {
        return Operation::Surface(json!({"close":true}));
    }
    let Sheet::Rename { id, text } = &mut cursor.sheet else {
        return Operation::None;
    };
    match key {
        "enter" => return Operation::Rename(id.clone(), text.clone()),
        "backspace" => {
            text.pop();
        }
        "ctrl+u" => text.clear(),
        "space" => text.push(' '),
        ch if ch.chars().count() == 1 => text.push_str(ch),
        _ => {}
    }
    Operation::None
}

fn escape(cursor: &mut Cursor, props: &SessionsProps) -> Operation {
    if !matches!(cursor.sheet, Sheet::None) {
        cursor.sheet = Sheet::None;
        return Operation::None;
    }
    if cursor.searching || !props.query.is_empty() {
        cursor.searching = false;
        return Operation::Surface(json!({"edit":""}));
    }
    Operation::Surface(json!({"close":true}))
}
fn activate(cursor: &mut Cursor, target: Option<Target>) -> Operation {
    match target {
        Some(Target::Session(id)) => Operation::Surface(json!({"resume":id})),
        Some(Target::More) => Operation::Surface(json!({"more":true})),
        Some(Target::Group(group)) => {
            if !cursor.expanded.remove(&group) {
                cursor.expanded.insert(group);
            }
            Operation::None
        }
        None => Operation::None,
    }
}

fn preview(cursor: &mut Cursor, target: Option<Target>) -> Operation {
    if let Some(Target::Session(id)) = target {
        cursor.sheet = Sheet::Loading(id.clone());
        Operation::Preview(id)
    } else {
        Operation::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn row_and_key_table() {
        let props = crate::tests::props();
        let id = SessionId::from_raw("row");
        for (key, expected) in [
            ("r", Operation::None),
            ("d", Operation::Delete(id.clone())),
            ("f", Operation::Fork(id.clone())),
            ("enter", Operation::Surface(json!({"resume":id}))),
            ("right", Operation::Preview(id.clone())),
        ] {
            assert_eq!(
                operation(
                    &mut Cursor::default(),
                    Some(Target::Session(id.clone())),
                    3,
                    key,
                    &props
                ),
                expected
            );
        }
        assert_eq!(
            operation(
                &mut Cursor::default(),
                Some(Target::More),
                1,
                "enter",
                &props
            ),
            Operation::Surface(json!({"more":true}))
        );
        let mut cursor = Cursor::default();
        operation(
            &mut cursor,
            Some(Target::Group("schedule".into())),
            1,
            "enter",
            &props,
        );
        assert!(cursor.expanded.contains("schedule"));
        operation(&mut cursor, None, 1, "/", &props);
        assert_eq!(
            operation(&mut cursor, Some(Target::Session(id)), 1, "f", &props),
            Operation::Unhandled
        );
    }
    #[tokio::test]
    async fn rename_reports_a_foreign_lock_without_opening() {
        let host = testing::TestHost::default();
        host.on_reconfigure(|_, _| async {
            Err(KernelError::new(
                ErrorCode::SessionLocked,
                "session is open elsewhere",
            ))
        });
        let error = crate::verbs::rename(&host.handle(), &SessionId::from_raw("row"), "renamed")
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::SessionLocked);
        assert!(error.message.contains("open elsewhere"));
        assert!(matches!(
            host.calls().as_slice(),
            [testing::HostCall::Reconfigure { .. }]
        ));
    }
    #[test]
    fn a_refresh_rebinds_selection_by_id_and_a_missing_row_is_inert() {
        let shared = std::sync::Arc::new(std::sync::Mutex::new(crate::state::State::default()));
        let handler = Handler(shared.clone());
        let mut props = crate::tests::props();
        let id = SessionId::from_raw("chosen");
        props.sessions = vec![
            crate::tests::summary("other", None),
            crate::tests::summary("chosen", None),
        ];
        shared
            .lock()
            .unwrap()
            .cursors
            .entry((props.session.clone(), "list".into()))
            .or_default()
            .selected = Some(Target::Session(id.clone()));
        assert_eq!(
            handler.press("list", "f", &props).unwrap(),
            Operation::Fork(id.clone())
        );
        props.sessions.reverse();
        assert_eq!(
            handler.press("list", "f", &props).unwrap(),
            Operation::Fork(id.clone())
        );
        assert_eq!(handler.press("list", "r", &props).unwrap(), Operation::None);
        props.sessions.retain(|s| s.id != id);
        for key in ["r", "f", "d", "enter", "right", "down"] {
            assert_eq!(handler.press("list", key, &props).unwrap(), Operation::None);
        }
        handler.press("list", "esc", &props).unwrap();
        assert_eq!(handler.press("list", "f", &props).unwrap(), Operation::None);
    }

    #[test]
    fn inline_edit_keeps_its_target_and_escape_cancels() {
        let id = SessionId::from_raw("chosen");
        let mut cursor = Cursor {
            sheet: Sheet::Rename {
                id: id.clone(),
                text: "old".into(),
            },
            ..Default::default()
        };
        edit(&mut cursor, "ctrl+u");
        edit(&mut cursor, "字");
        edit(&mut cursor, "space");
        edit(&mut cursor, "x");
        assert_eq!(
            edit(&mut cursor, "enter"),
            Operation::Rename(id, "字 x".into())
        );
        assert_eq!(edit(&mut cursor, "esc"), Operation::None);
        assert!(matches!(cursor.sheet, Sheet::None));
    }
}
