use crate::{Cursor, Shared, rows::Target};
use bingo_sdk::*;
use bingo_services::{experience, memory};
use serde_json::Value;

#[derive(Debug, PartialEq)]
enum Operation {
    Forget(Target),
    Refuse(Target),
    Close,
    Handled(bool),
}
fn press(cursor: &mut Cursor, target: Option<Target>, key: &str) -> Operation {
    match key {
        "up" | "down" => {
            let rows = cursor
                .groups
                .iter()
                .flat_map(|g| &g.rows)
                .map(|r| pick::Row::new(r.target.id(), pick::text("", "fg")))
                .collect::<Vec<_>>();
            cursor.selected = pick::moved(&rows, cursor.selected.as_deref(), key == "down");
        }
        "esc" | "ctrl+c" => return Operation::Close,
        "backspace" => match target {
            Some(target @ Target::Instruction(_)) => return Operation::Refuse(target),
            Some(target) => return Operation::Forget(target),
            None => {}
        },
        _ => {}
    }
    Operation::Handled(true)
}
pub struct Handler(pub Shared, pub std::path::PathBuf);
#[async_trait::async_trait]
impl Command for Handler {
    fn spec(&self) -> CommandSpec {
        CommandSpec::ui(crate::ID)
    }
    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let args: Value = serde_json::from_str(args).map_err(crate::error)?;
        if args["event"] != "press" || args["handler"] != crate::REQUEST {
            return Ok(CommandOutcome::Handled { handled: false });
        }
        let press_value = sheet::pressed(&args);
        if press_value.is_none() {
            return Ok(CommandOutcome::Handled { handled: false });
        }
        let picked = press_value.as_ref().and_then(sheet::Press::picked);
        let key = press_value
            .as_ref()
            .and_then(sheet::Press::key)
            .unwrap_or("enter");
        let target: Option<Target> =
            serde_json::from_value(args["target"].clone()).map_err(crate::error)?;
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
            if let Some(id) = picked
                && targets.iter().any(|target| target.id() == id)
            {
                cursor.selected = Some(id.into());
            }
            let selected = targets
                .iter()
                .find(|t| Some(t.id()) == cursor.selected)
                .cloned()
                .or_else(|| targets.first().cloned());
            if target != selected && picked.is_none() {
                return Ok(CommandOutcome::Handled { handled: true });
            }
            press(cursor, selected, key)
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
        let title = self.title(&target, cx);
        let result = forget(target, cx).await;
        let groups = crate::panel::groups(&cx.host, &cx.cwd, &self.1).await?;
        if let Some(cursor) = self.0.lock().map_err(crate::error)?.get_mut(&cx.session) {
            retain_selection(cursor, groups);
        }
        match result {
            Ok(_) => {
                let said = toast::Say::ok(format!("forgot · {title}"));
                Ok(said.outcome(said.plain()))
            }
            Err(error) if error.code == ErrorCode::NotFound => Ok(CommandOutcome::Applied {
                message: Some("Already gone; nothing forgotten.".into()),
            }),
            Err(error) => Err(error),
        }
    }
    async fn apply(
        &self,
        operation: Operation,
        cx: &CommandContext,
    ) -> Result<CommandOutcome, KernelError> {
        match operation {
            Operation::Handled(handled) => Ok(CommandOutcome::Handled { handled }),
            Operation::Refuse(target) => {
                let said = toast::Say::new([
                    "instructions live in files".into(),
                    format!("edit {}", self.title(&target, cx)),
                ]);
                Ok(said.outcome(said.plain()))
            }
            Operation::Close => {
                self.0.lock().map_err(crate::error)?.remove(&cx.session);
                cx.host
                    .signal(&cx.session, crate::ID, crate::REQUEST, Value::Null)
                    .await?;
                Ok(CommandOutcome::Handled { handled: true })
            }
            Operation::Forget(target) => self.forget(target, cx).await,
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
fn retain_selection(cursor: &mut crate::Cursor, groups: Vec<crate::rows::Group>) {
    let ids = groups
        .iter()
        .flat_map(|group| &group.rows)
        .map(|row| row.target.id())
        .collect::<Vec<_>>();
    if !ids.iter().any(|id| Some(id) == cursor.selected.as_ref()) {
        let previous = cursor
            .groups
            .iter()
            .flat_map(|group| &group.rows)
            .map(|row| row.target.id())
            .collect::<Vec<_>>();
        let at = previous
            .iter()
            .position(|id| Some(id) == cursor.selected.as_ref())
            .unwrap_or(0);
        cursor.selected = previous
            .iter()
            .skip(at + 1)
            .chain(previous[..at].iter().rev())
            .find(|id| ids.contains(id))
            .cloned()
            .or_else(|| ids.first().cloned());
    }
    cursor.groups = groups;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn memory_rows_come_from_the_builder_and_letters_do_nothing() {
        let view = crate::draw::tree(&[], None, 18, 100, Default::default());
        bingo_sdk::testing::assert_separate_hints(&view);
        assert!(view.fold().contains("Memory"));
        let mut cursor = Cursor::default();
        assert_eq!(press(&mut cursor, None, "d"), Operation::Handled(true));
        let target = Target::Instruction("AGENTS.md".into());
        assert_eq!(
            press(&mut cursor, Some(target.clone()), "backspace"),
            Operation::Refuse(target)
        );
    }
}

#[cfg(test)]
mod selection_tests {
    use super::*;
    fn group(names: &[&str]) -> Vec<crate::rows::Group> {
        vec![crate::rows::Group {
            name: "instructions",
            rows: names
                .iter()
                .map(|name| crate::rows::Row {
                    label: (*name).into(),
                    note: String::new(),
                    tokens: 0,
                    target: Target::Instruction((*name).into()),
                })
                .collect(),
        }]
    }
    #[test]
    fn forgetting_selects_the_next_surviving_identity_then_the_previous() {
        let mut cursor = crate::Cursor {
            groups: group(&["a", "b", "c"]),
            selected: Some("b".into()),
        };
        retain_selection(&mut cursor, group(&["a", "c"]));
        assert_eq!(cursor.selected.as_deref(), Some("c"));
        retain_selection(&mut cursor, group(&["a"]));
        assert_eq!(cursor.selected.as_deref(), Some("a"));
    }
}
