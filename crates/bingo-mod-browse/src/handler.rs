use crate::{
    Cursor, Shared,
    keys::{self, Operation},
};
use bingo_sdk::{component::BrowseProps, *};
use serde_json::{Value, json};
pub struct Open;
#[async_trait::async_trait]
impl Command for Open {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            name: "browse".into(),
            aliases: vec![],
            hint: "read and search conversation turns".into(),
            args: ArgSpec::None,
            instant: true,
            family: "terminal".into(),
            hidden: false,
            keys: vec!["ctrl+f".into()],
        }
    }
    async fn run(&self, _: &str, _: &CommandContext) -> Result<CommandOutcome, KernelError> {
        Ok(CommandOutcome::Surface {
            action: Action {
                name: "Browse".into(),
                args: json!({}),
            },
            fallback: Some("/browse is terminal only.".into()),
        })
    }
}
pub struct Handler(pub Shared);
#[async_trait::async_trait]
impl Command for Handler {
    fn spec(&self) -> CommandSpec {
        CommandSpec::ui("browse")
    }
    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let args: Value = serde_json::from_str(args).map_err(crate::error)?;
        if args["event"] != "press" {
            return Ok(CommandOutcome::Handled { handled: false });
        }
        let props: BrowseProps =
            serde_json::from_value(args["props"].clone()).map_err(crate::error)?;
        let request = args["handler"].as_str().unwrap_or_default();
        let key = args["value"]["key"].as_str().unwrap_or_default();
        let id = (cx.session.clone(), request.to_owned());
        let mut cursor = self
            .0
            .lock()
            .map_err(crate::error)?
            .get(&id)
            .cloned()
            .unwrap_or_else(|| initial(&props));
        let operation = keys::press(&mut cursor, &props, key);
        if operation == Operation::Preview {
            cursor.sheet = Some(preview(&cx.host, &props, &cx.cwd).await?);
            cursor.mode = Default::default();
        }
        let value = match operation {
            Operation::Close => json!({"close":true}),
            Operation::Run(line) => json!({"run":line}),
            Operation::Pager(item) => json!({"pager":item}),
            Operation::Copy(item) => json!({"copy":item}),
            _ => Value::Null,
        };
        {
            let mut state = self.0.lock().map_err(crate::error)?;
            if value.get("close").is_some()
                || value.get("run").is_some()
                || value.get("pager").is_some()
            {
                state.remove(&id);
            } else {
                state.insert(id, cursor);
            }
        }
        cx.host
            .invalidate(&cx.session, vec![request.into()])
            .await?;
        if value.is_null() {
            return Ok(CommandOutcome::Handled { handled: true });
        }
        let mut action = props.action;
        action.args["value"] = value;
        Ok(CommandOutcome::Surface {
            action,
            fallback: None,
        })
    }
}
pub(crate) fn initial(props: &BrowseProps) -> Cursor {
    Cursor {
        selected: props.selected.clone(),
        query: props.query.clone(),
        current: props.current,
        ..Default::default()
    }
}
pub(crate) async fn preview(
    host: &HostHandle,
    props: &BrowseProps,
    cwd: &std::path::Path,
) -> Result<bingo_services::checkpoints::Touched, KernelError> {
    let turn = props
        .selected
        .as_ref()
        .ok_or_else(|| KernelError::new(ErrorCode::InvalidInput, "there is no turn to rewind"))?;
    let service = bingo_services::checkpoints::find(host).ok_or_else(|| {
        KernelError::new(ErrorCode::NotReady, "checkpoint preview is unavailable")
    })?;
    let mut touched = service
        .touched(&props.session, turn)
        .await
        .map_err(crate::error)?;
    for file in &mut touched.files {
        if let Ok(relative) = file.path.strip_prefix(cwd) {
            file.path = relative.to_path_buf();
        }
    }
    Ok(touched)
}
