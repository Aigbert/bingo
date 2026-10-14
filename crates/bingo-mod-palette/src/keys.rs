use crate::{Shared, rows::Target};
use bingo_sdk::{component::PaletteProps, *};
use serde_json::{Value, json};
pub struct Open(pub &'static str);
#[async_trait::async_trait]
impl Command for Open {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            name: self.0.into(),
            aliases: vec![],
            hint: if self.0 == "help" {
                "keys, in this list"
            } else {
                "reopen a session"
            }
            .into(),
            args: ArgSpec::None,
            instant: true,
            family: "session".into(),
            hidden: false,
            keys: if self.0 == "resume" {
                vec!["ctrl+r".into()]
            } else {
                vec![]
            },
        }
    }
    async fn run(&self, args: &str, _: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let value = if self.0 == "resume" && !args.trim().is_empty() {
            json!({"resume":args.trim()})
        } else {
            json!({"open":if self.0=="help"{"keys"}else{"sessions"}})
        };
        Ok(CommandOutcome::Surface {
            action: Action {
                name: "Palette".into(),
                args: value,
            },
        })
    }
}
pub struct Handler(pub Shared);
#[async_trait::async_trait]
impl Command for Handler {
    fn spec(&self) -> CommandSpec {
        CommandSpec::ui("palette")
    }
    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let args: Value = serde_json::from_str(args).map_err(crate::error)?;
        let request = args["handler"].as_str().unwrap_or_default();
        let key = args["value"]["key"].as_str().unwrap_or_default();
        if args["event"] != "press" {
            return Ok(CommandOutcome::Handled { handled: false });
        }
        let props: PaletteProps =
            serde_json::from_value(args["props"].clone()).map_err(crate::error)?;
        let outcome = self.press(request, key, &props)?;
        cx.host
            .invalidate(&cx.session, vec![request.into()])
            .await?;
        Ok(outcome)
    }
}
impl Handler {
    fn press(
        &self,
        request: &str,
        key: &str,
        props: &PaletteProps,
    ) -> Result<CommandOutcome, KernelError> {
        let mut shared = self.0.lock().map_err(crate::error)?;
        let (groups, initial) = crate::draw::groups(props, shared.recent.names());
        let rows = groups
            .iter()
            .flat_map(|group| &group.rows)
            .collect::<Vec<_>>();
        let cursor = shared
            .cursors
            .entry((props.summary.id.clone(), request.into()))
            .or_default();
        if cursor.query != props.query {
            cursor.query = props.query.clone();
            cursor.selected = initial;
        }
        let target = rows.get(cursor.selected).map(|row| row.target.clone());
        let (handled, operation) = operation(cursor, rows.len(), target, key, props);
        if !handled {
            return Ok(CommandOutcome::Handled { handled: false });
        }
        let Some(operation) = operation else {
            return Ok(CommandOutcome::Handled { handled: true });
        };
        if let Some(line) = operation["run"].as_str() {
            shared.recent.remember(line).map_err(crate::error)?;
        }
        if operation.get("close").is_some()
            || operation.get("run").is_some()
            || operation.get("resume").is_some()
        {
            shared
                .cursors
                .remove(&(props.summary.id.clone(), request.into()));
        }
        surface(props, operation)
    }
}
fn surface(props: &PaletteProps, operation: Value) -> Result<CommandOutcome, KernelError> {
    let mut action = props.action.clone();
    if !action.args.is_object() {
        return Ok(CommandOutcome::Handled { handled: false });
    }
    action.args["value"] = operation;
    Ok(CommandOutcome::Surface { action })
}

fn activate(target: Target, props: &PaletteProps) -> Value {
    match target {
        Target::Command(name) => {
            let spec = props
                .commands
                .iter()
                .find(|entry| entry.id == name)
                .and_then(|entry| serde_json::from_value::<CommandSpec>(entry.meta.clone()).ok());
            if spec.is_some_and(|spec| !matches!(spec.args, ArgSpec::None)) {
                json!({"edit":format!("/{name} ")})
            } else {
                json!({"run":format!("/{name}")})
            }
        }
        Target::Key(line) if line.starts_with("@key:") => {
            json!({"key":line.trim_start_matches("@key:")})
        }
        Target::Key(line) | Target::Run(line) => json!({"run":line}),
        Target::Edit(line) => json!({"edit":line}),
        Target::Session(id) if id == props.summary.id.as_str() => {
            json!({"message":"this session is already in view"})
        }
        Target::Session(id) => json!({"resume":id}),
    }
}

fn operation(
    cursor: &mut crate::Cursor,
    count: usize,
    target: Option<Target>,
    key: &str,
    props: &PaletteProps,
) -> (bool, Option<Value>) {
    let operation = match key {
        "up" => {
            cursor.selected = cursor.selected.saturating_sub(1);
            None
        }
        "down" => {
            cursor.selected = (cursor.selected + 1).min(count.saturating_sub(1));
            None
        }
        "esc" => Some(
            crate::args::parent(&props.query, &props.commands)
                .map(|edit| json!({"edit":edit}))
                .unwrap_or_else(|| json!({"close":true})),
        ),
        "backspace" if props.query.is_empty() || props.query == "/" => Some(json!({"close":true})),
        "tab" => target.and_then(|target| match target {
            Target::Command(name) => Some(json!({"edit":format!("/{name} ")})),
            _ => None,
        }),
        "enter" => target.map(|target| activate(target, props)).or_else(|| {
            (props.query.starts_with('/') && props.query.len() > 1)
                .then(|| json!({"run":props.query.trim()}))
        }),
        _ => return (false, None),
    };
    (true, operation)
}
