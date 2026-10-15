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
            load_preview(&mut cursor, &props, cx).await?;
        }
        let value = match operation {
            Operation::Close => json!({"close":true}),
            Operation::Notice(message) => json!({"notice":message}),
            Operation::Run(line) => json!({"run":line}),
            Operation::Pager(item) => json!({"pager":item}),
            Operation::Copy(item) => json!({"copy":item}),
            _ => Value::Null,
        };
        {
            let mut state = self.0.lock().map_err(crate::error)?;
            if value.get("close").is_some() || value.get("run").is_some() {
                state.remove(&id);
            } else {
                state.insert(id, cursor);
            }
        }
        outcome(props.action, value, request, cx).await
    }
}
async fn load_preview(
    cursor: &mut Cursor,
    props: &BrowseProps,
    cx: &CommandContext,
) -> Result<(), KernelError> {
    let mut target = props.clone();
    target.selected = cursor
        .sheet
        .as_ref()
        .and_then(|sheet| sheet.turns.first().map(|turn| turn.id.clone()))
        .or_else(|| cursor.selected.clone());
    cursor.sheet = preview(&cx.host, &target, &cx.cwd).await?;
    cursor.sheet_top = 0;
    Ok(())
}
async fn outcome(
    mut action: Action,
    value: Value,
    request: &str,
    cx: &CommandContext,
) -> Result<CommandOutcome, KernelError> {
    if value.is_null() {
        cx.host
            .invalidate(&cx.session, vec![request.into()])
            .await?;
        return Ok(CommandOutcome::Handled { handled: true });
    }
    action.args["value"] = value;
    Ok(CommandOutcome::Surface {
        action,
        fallback: None,
    })
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
) -> Result<Option<crate::Preview>, KernelError> {
    if props.busy {
        return Ok(None);
    }
    let Some(at) = props
        .turns
        .iter()
        .position(|turn| Some(&turn.id) == props.selected.as_ref())
    else {
        return Ok(None);
    };
    let service = bingo_services::checkpoints::find(host).ok_or_else(|| {
        KernelError::new(ErrorCode::NotReady, "checkpoint preview is unavailable")
    })?;
    let Some(mut touched) = service
        .touched(&props.session, &props.turns[at].id)
        .await
        .map_err(crate::error)?
    else {
        return Ok(None);
    };
    for file in &mut touched.files {
        if let Ok(relative) = file.path.strip_prefix(cwd) {
            file.path = relative.to_path_buf();
        }
    }
    Ok(Some(crate::Preview {
        turns: props.turns[at..].to_vec(),
        length: props.length,
        touched,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_services::checkpoints::{Checkpoints, KEY, Touched};
    use std::{future::Future, sync::Arc, task};

    struct Answer(Result<Option<Touched>, String>);
    #[async_trait::async_trait]
    impl Checkpoints for Answer {
        async fn touched(
            &self,
            _: &SessionId,
            _: &TurnId,
        ) -> Result<Option<Touched>, ServiceError> {
            self.0.clone().map_err(ServiceError)
        }
    }

    /// The double answers at once, so one poll finishes the preview.
    fn previewed(answer: Result<Option<Touched>, String>) -> Result<bool, KernelError> {
        let service: Arc<dyn Checkpoints> = Arc::new(Answer(answer));
        let host = bingo_sdk::testing::ServiceHost::holding(&KEY, service);
        let props = crate::keys::tests::props();
        let mut future = Box::pin(preview(&host, &props, std::path::Path::new("/")));
        let mut cx = task::Context::from_waker(task::Waker::noop());
        match future.as_mut().poll(&mut cx) {
            task::Poll::Ready(sheet) => sheet.map(|sheet| sheet.is_some()),
            task::Poll::Pending => panic!("the double answers at once"),
        }
    }

    #[test]
    fn a_missing_turn_is_no_sheet_and_a_failure_is_an_error() {
        assert!(previewed(Ok(Some(Touched::default()))).unwrap());
        assert!(!previewed(Ok(None)).unwrap());
        assert!(previewed(Err("no such turn".into())).is_err());
    }
}
