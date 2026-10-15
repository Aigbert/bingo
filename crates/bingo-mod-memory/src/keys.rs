use crate::{Cursor, Shared, rows::Target};
use bingo_sdk::*;
use bingo_services::{experience, memory};
use serde_json::{Value, json};

#[derive(Debug, PartialEq)]
enum Operation {
    Read(Target),
    Forget(Target),
    Refuse,
    Close,
    Handled(bool),
}
fn press(
    cursor: &mut Cursor,
    count: usize,
    target: Option<Target>,
    confirming: Option<Target>,
    key: &str,
) -> Operation {
    match key {
        "up" => {
            cursor.selected = cursor.selected.saturating_sub(1);
            cursor.confirming = None;
        }
        "down" => {
            cursor.selected = (cursor.selected + 1).min(count.saturating_sub(1));
            cursor.confirming = None;
        }
        "esc" if cursor.confirming.is_some() => {
            cursor.confirming = None;
        }
        "esc" | "ctrl+c" => return Operation::Close,
        "d" => match target {
            Some(Target::Instruction(_)) => return Operation::Refuse,
            target => cursor.confirming = target,
        },
        "enter" => {
            if let Some(confirmed) = confirming {
                if cursor.confirming.as_ref() == Some(&confirmed) {
                    cursor.confirming = None;
                    return Operation::Forget(confirmed);
                }
            } else if cursor.confirming.is_none()
                && let Some(target) = target
            {
                return Operation::Read(target);
            }
        }
        _ => return Operation::Handled(false),
    }
    Operation::Handled(true)
}
pub struct Handler(pub Shared, pub std::path::PathBuf);
#[async_trait::async_trait]
impl Command for Handler {
    fn spec(&self) -> CommandSpec {
        CommandSpec::ui("memory")
    }
    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let args: Value = serde_json::from_str(args).map_err(crate::error)?;
        if args["event"] != "press" || args["handler"] != crate::REQUEST {
            return Ok(CommandOutcome::Handled { handled: false });
        }
        let target: Option<Target> =
            serde_json::from_value(args["target"].clone()).map_err(crate::error)?;
        let confirming: Option<Target> =
            serde_json::from_value(args["confirming"].clone()).map_err(crate::error)?;
        let operation = {
            let mut shared = self.0.lock().map_err(crate::error)?;
            let Some(cursor) = shared.get_mut(&cx.session) else {
                return Ok(CommandOutcome::Handled { handled: true });
            };
            let targets = cursor
                .groups
                .iter()
                .flat_map(|group| &group.rows)
                .map(|row| row.target.clone())
                .collect::<Vec<_>>();
            let selected = targets.get(cursor.selected).cloned();
            if target != selected
                || confirming
                    .as_ref()
                    .is_some_and(|target| Some(target) != selected.as_ref())
            {
                return Ok(CommandOutcome::Handled { handled: true });
            }
            press(
                cursor,
                targets.len(),
                selected,
                confirming,
                args["value"]["key"].as_str().unwrap_or_default(),
            )
        };
        let outcome = self.apply(operation, cx).await;
        cx.host
            .invalidate(&cx.session, vec![crate::REQUEST.into()])
            .await?;
        outcome
    }
}
impl Handler {
    fn title(&self, target: &Target, cx: &CommandContext) -> String {
        match target {
            Target::Instruction(path) => crate::rows::path_label(path, &self.1, &cx.cwd),
            _ => target.name(),
        }
    }
    async fn forget(
        &self,
        target: Target,
        cx: &CommandContext,
    ) -> Result<CommandOutcome, KernelError> {
        let result = forget(target, cx).await;
        let groups = crate::panel::groups(&cx.host, &cx.cwd, &self.1).await?;
        if let Some(cursor) = self.0.lock().map_err(crate::error)?.get_mut(&cx.session) {
            cursor.groups = groups;
            let count = cursor
                .groups
                .iter()
                .map(|group| group.rows.len())
                .sum::<usize>();
            cursor.selected = cursor.selected.min(count.saturating_sub(1));
            cursor.confirming = None;
        }
        Ok(CommandOutcome::Applied {
            message: Some(match result {
                Ok(_) => "Forgotten. Prompt refreshes later; say not to relearn it.".into(),
                Err(error) if error.code == ErrorCode::NotFound => {
                    "Already gone; nothing forgotten.".into()
                }
                Err(error) => return Err(error),
            }),
        })
    }
    async fn apply(
        &self,
        operation: Operation,
        cx: &CommandContext,
    ) -> Result<CommandOutcome, KernelError> {
        match operation {
            Operation::Handled(handled) => Ok(CommandOutcome::Handled { handled }),
            Operation::Refuse => Ok(CommandOutcome::Applied {
                message: Some("edit the file itself".into()),
            }),
            Operation::Close => {
                self.0.lock().map_err(crate::error)?.remove(&cx.session);
                cx.host
                    .signal(&cx.session, crate::ID, crate::REQUEST, Value::Null)
                    .await?;
                Ok(CommandOutcome::Handled { handled: true })
            }
            Operation::Read(target) => {
                let text = read(&target, cx).await?;
                Ok(CommandOutcome::Surface {
                    action: Action {
                        name: "Pager".into(),
                        args: json!({
                            "title": self.title(&target, cx),
                            "text": crate::fence::document(&text),
                            "return_to": {"plugin": crate::ID, "id": crate::REQUEST},
                        }),
                    },
                    fallback: Some(text),
                })
            }
            Operation::Forget(target) => self.forget(target, cx).await,
        }
    }
}
async fn read(target: &Target, cx: &CommandContext) -> Result<String, KernelError> {
    match target {
        Target::Memory { path, .. } | Target::Instruction(path) => {
            memory::find(&cx.host)
                .ok_or_else(crate::unavailable)?
                .read(memory::Read {
                    cwd: cx.cwd.clone(),
                    path: path.clone(),
                })
                .await
        }
        Target::Experience(id) => {
            experience::find(&cx.host)
                .ok_or_else(crate::unavailable)?
                .read(experience::Read {
                    cwd: cx.cwd.clone(),
                    id: id.clone(),
                })
                .await
        }
    }
}
async fn forget(target: Target, cx: &CommandContext) -> Result<String, KernelError> {
    match target {
        Target::Memory { scope, name, .. } => {
            memory::find(&cx.host)
                .ok_or_else(crate::unavailable)?
                .forget(memory::Forget {
                    cwd: cx.cwd.clone(),
                    scope: Some(scope),
                    name,
                })
                .await
        }
        Target::Experience(id) => {
            experience::find(&cx.host)
                .ok_or_else(crate::unavailable)?
                .forget(experience::Forget {
                    cwd: cx.cwd.clone(),
                    id,
                })
                .await
        }
        Target::Instruction(_) => Err(KernelError::new(
            ErrorCode::InvalidInput,
            "edit the file itself",
        )),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keys_read_confirm_cancel_and_refuse_instructions() {
        for target in [
            Target::Experience("a".into()),
            Target::Memory {
                scope: memory::Scope::User,
                name: "a".into(),
                path: "a.md".into(),
            },
        ] {
            let mut cursor = Cursor::default();
            assert_eq!(
                press(&mut cursor, 2, Some(target.clone()), None, "enter"),
                Operation::Read(target.clone())
            );
            assert_eq!(
                press(&mut cursor, 2, Some(target.clone()), None, "d"),
                Operation::Handled(true)
            );
            assert_eq!(
                press(&mut cursor, 2, Some(target.clone()), None, "enter"),
                Operation::Handled(true),
                "an old drawing cannot confirm"
            );
            assert_eq!(
                press(
                    &mut cursor,
                    2,
                    Some(target.clone()),
                    Some(target.clone()),
                    "enter"
                ),
                Operation::Forget(target.clone())
            );
            press(&mut cursor, 2, Some(target.clone()), None, "d");
            press(
                &mut cursor,
                2,
                Some(target.clone()),
                Some(target.clone()),
                "esc",
            );
            assert!(cursor.confirming.is_none());
            assert_eq!(press(&mut cursor, 2, None, None, "esc"), Operation::Close);
        }
        let mut cursor = Cursor::default();
        assert_eq!(
            press(
                &mut cursor,
                1,
                Some(Target::Instruction("AGENTS.md".into())),
                None,
                "d"
            ),
            Operation::Refuse
        );
        for _ in 0..4 {
            press(&mut cursor, 2, None, None, "down");
        }
        assert_eq!(cursor.selected, 1);
        for _ in 0..4 {
            press(&mut cursor, 2, None, None, "up");
        }
        assert_eq!(cursor.selected, 0);
        assert_eq!(
            press(&mut cursor, 0, None, None, "enter"),
            Operation::Handled(true)
        );
    }
}
