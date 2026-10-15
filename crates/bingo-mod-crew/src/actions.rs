use crate::{
    keys::Operation,
    state::{Panel, Rows},
};
use bingo_sdk::*;
use bingo_services::{agents_control as agents, rooms_control as rooms, tasks_write as tasks};
use serde_json::{Value, json};
pub struct Handler(pub crate::Shared);
#[async_trait::async_trait]
impl Command for Handler {
    fn spec(&self) -> CommandSpec {
        CommandSpec::ui(crate::ID)
    }
    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let args: Value = serde_json::from_str(args).map_err(crate::error)?;
        if args["handler"] != crate::REQUEST {
            return Ok(handled());
        }
        if args["event"] == "close" {
            crate::command::close(cx, &self.0).await?;
            return Ok(handled());
        }
        let rows = crate::state::read(&cx.host, &cx.session).await?;
        let Some(mut panel) = self
            .0
            .lock()
            .map_err(crate::error)?
            .get(&cx.session)
            .cloned()
        else {
            return Ok(handled());
        };
        let key = args["value"]["key"].as_str().unwrap_or_default();
        crate::state::clamp(&mut panel, &rows);
        if key != "esc"
            && (args["row"] != rows.identity(panel.tab, panel.selected)
                || args["tab"] != panel.tab.label())
        {
            return Ok(handled());
        }
        let operation = crate::keys::operation(&panel, &rows, key);
        let outcome = tokio::select! {
            _ = cx.cancel.cancelled() => Err(crate::error("crew action interrupted")),
            result = act(operation, &mut panel, &rows, cx, &self.0) => result,
        };
        if let Some(current) = self.0.lock().map_err(crate::error)?.get_mut(&cx.session) {
            current.apply_input(&panel);
        }
        cx.host
            .invalidate(&cx.session, vec![crate::REQUEST.into()])
            .await?;
        match outcome {
            Ok(outcome) => Ok(outcome),
            Err(error) => Ok(CommandOutcome::Acknowledged {
                result: json!({"message":error.message,"handled":true}),
            }),
        }
    }
}
fn handled() -> CommandOutcome {
    CommandOutcome::Handled { handled: true }
}
async fn act(
    op: Operation,
    panel: &mut Panel,
    rows: &Rows,
    cx: &CommandContext,
    shared: &crate::Shared,
) -> Result<CommandOutcome, KernelError> {
    match op {
        Operation::Close if panel.confirm.is_some() => {
            panel.confirm = None;
        }
        Operation::Close => crate::command::close(cx, shared).await?,
        Operation::Tab => next_tab(panel, rows),
        Operation::Move(down) => move_row(panel, rows, down),
        Operation::Model => panel.model = !panel.model,
        Operation::Confirm(name) => {
            person()?;
            panel.confirm = Some(name);
        }
        Operation::Fill(text) => {
            crate::command::close(cx, shared).await?;
            return Ok(surface("Composer", json!({"text":text})));
        }
        Operation::View(id) => {
            crate::command::close(cx, shared).await?;
            return Ok(surface("View", json!({"view":id})));
        }
        Operation::Pick(id) => {
            person()?;
            crate::command::close(cx, shared).await?;
            return crate::dispatch::model(cx, &id).await;
        }
        op @ (Operation::Stop(_) | Operation::CloseRoom(_) | Operation::Dismiss(_)) => {
            let outcome = write(op, cx).await?;
            panel.confirm = None;
            return Ok(outcome);
        }
        op => return write(op, cx).await,
    }
    Ok(handled())
}
fn next_tab(panel: &mut Panel, rows: &Rows) {
    let tabs = rows.tabs();
    let at = tabs.iter().position(|t| *t == panel.tab).unwrap_or(0);
    panel.tab = tabs
        .get((at + 1) % tabs.len().max(1))
        .copied()
        .unwrap_or_default();
    panel.selected = 0;
    panel.model = false;
    panel.follow = None;
}
fn move_row(panel: &mut Panel, rows: &Rows, down: bool) {
    panel.selected = if down {
        (panel.selected + 1).min(rows.len(panel.tab).saturating_sub(1))
    } else {
        panel.selected.saturating_sub(1)
    };
    panel.model = false;
    panel.follow = None;
}

fn person() -> Result<(), KernelError> {
    if render::asked() && render::vouched() {
        Ok(())
    } else {
        Err(crate::error(
            "Use the crew pane's own control or type this command yourself.",
        ))
    }
}
async fn write(op: Operation, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
    let session = cx.session.clone();
    let message = match op {
        Operation::Stop(name) => {
            person()?;
            agents::find(&cx.host)
                .ok_or_else(|| crate::error("agents unavailable"))?
                .stop(agents::Named { session, name })
                .await
        }
        Operation::Dismiss(name) => {
            person()?;
            agents::find(&cx.host)
                .ok_or_else(|| crate::error("agents unavailable"))?
                .dismiss(agents::Named { session, name })
                .await
        }
        Operation::CloseRoom(name) => {
            person()?;
            rooms::find(&cx.host)
                .ok_or_else(|| crate::error("rooms unavailable"))?
                .close(rooms::Named { session, name })
                .await
        }
        Operation::Tick(id, status) => {
            tasks::find(&cx.host)
                .ok_or_else(|| crate::error("tasks unavailable"))?
                .set_status(tasks::SetStatus {
                    session,
                    id,
                    status,
                })
                .await
        }
        _ => return Ok(handled()),
    }
    .map_err(crate::error)?;
    Ok(CommandOutcome::Acknowledged {
        result: json!({"message":message,"handled":true}),
    })
}
fn surface(name: &str, args: Value) -> CommandOutcome {
    CommandOutcome::Surface {
        action: Action {
            name: name.into(),
            args,
        },
        fallback: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn person_controls_require_both_asked_and_the_owners_drawing() {
        let host = testing::TestHost::default();
        let cx = CommandContext {
            session: SessionId::from_raw("ses_parent"),
            cwd: "/unused".into(),
            host: host.handle(),
            cancel: Default::default(),
        };
        for (asked, vouched) in [(false, false), (true, false), (false, true)] {
            for operation in [
                Operation::Pick(SessionId::from_raw("ses_child")),
                Operation::Stop("reviewer".into()),
                Operation::Dismiss("reviewer".into()),
                Operation::CloseRoom("design".into()),
                Operation::Confirm(crate::state::Confirmation::Dismiss("reviewer".into())),
            ] {
                let error = render::under_input(
                    asked,
                    render::under_press(
                        vouched,
                        act(
                            operation,
                            &mut Panel::default(),
                            &Rows::default(),
                            &cx,
                            &crate::Shared::default(),
                        ),
                    ),
                )
                .await
                .unwrap_err();
                assert!(error.message.contains("own control"), "{error}");
                assert!(host.calls().is_empty());
            }
        }
    }
    #[tokio::test]
    async fn dismissal_waits_for_confirmation_and_escape_cancels() {
        let host = testing::TestHost::default();
        let cx = CommandContext {
            session: SessionId::from_raw("ses_parent"),
            cwd: "/unused".into(),
            host: host.handle(),
            cancel: Default::default(),
        };
        let mut panel = Panel::default();
        let confirm = crate::state::Confirmation::Dismiss("reviewer".into());
        render::under_input(
            true,
            render::under_press(
                true,
                act(
                    Operation::Confirm(confirm.clone()),
                    &mut panel,
                    &Rows::default(),
                    &cx,
                    &crate::Shared::default(),
                ),
            ),
        )
        .await
        .unwrap();
        assert_eq!(panel.confirm, Some(confirm));
        assert!(host.calls().is_empty());
        act(
            Operation::Close,
            &mut panel,
            &Rows::default(),
            &cx,
            &crate::Shared::default(),
        )
        .await
        .unwrap();
        assert_eq!(panel.confirm, None);
        assert!(host.calls().is_empty());
    }
}
