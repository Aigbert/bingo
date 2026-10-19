//! A press on the crew sheet, answered: the sheet moves its pick, walks in,
//! or closes and writes a mention.
use crate::keys::Operation;
use bingo_sdk::*;
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
            return Ok(CommandOutcome::Handled { handled: false });
        }
        if args["event"] == "close" {
            crate::command::close(&cx.host, &cx.session, &self.0).await?;
            return Ok(handled());
        }
        let Some(press) = sheet::pressed(&args) else {
            return Ok(handled());
        };
        let crew = crate::read::read(&cx.host, &cx.session).await?;
        let operation = crate::keys::operation(&crew.entries, args["selected"].as_str(), &press);
        act(operation, cx, &self.0).await
    }
}

async fn act(
    operation: Operation,
    cx: &CommandContext,
    shared: &crate::Shared,
) -> Result<CommandOutcome, KernelError> {
    match operation {
        Operation::Close => crate::command::close(&cx.host, &cx.session, shared).await?,
        Operation::Move(selected) => {
            crate::command::publish(&cx.host, &cx.session, selected).await?;
        }
        Operation::Walk(id) => {
            crate::command::close(&cx.host, &cx.session, shared).await?;
            return Ok(surface("View", json!({"view": id})));
        }
        Operation::Mention(text) => {
            crate::command::close(&cx.host, &cx.session, shared).await?;
            return Ok(surface("Composer", json!({"text": text})));
        }
        Operation::None => {}
    }
    Ok(handled())
}

fn handled() -> CommandOutcome {
    CommandOutcome::Handled { handled: true }
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

    fn context(host: &testing::TestHost) -> CommandContext {
        CommandContext {
            surface: None,
            session: SessionId::from_raw("ses_parent"),
            cwd: "/unused".into(),
            host: host.handle(),
            cancel: Default::default(),
        }
    }

    #[tokio::test]
    async fn walking_in_and_mentioning_close_the_sheet_first() {
        let host = testing::TestHost::default();
        host.on_signal(|_, _, _, _| async { Ok(()) });
        let shared = crate::Shared::default();
        let cx = context(&host);
        shared
            .lock()
            .unwrap()
            .insert(cx.session.clone(), Default::default());
        let walked = act(
            Operation::Walk(SessionId::from_raw("ses_scout")),
            &cx,
            &shared,
        )
        .await
        .unwrap();
        let CommandOutcome::Surface { action, .. } = walked else {
            panic!("walking in is the surface's to do");
        };
        assert_eq!(
            (action.name.as_str(), &action.args),
            ("View", &json!({"view":"ses_scout"}))
        );
        assert!(shared.lock().unwrap().is_empty());
        let CommandOutcome::Surface { action, .. } =
            act(Operation::Mention("@ada ".into()), &cx, &shared)
                .await
                .unwrap()
        else {
            panic!("a mention is written in the composer");
        };
        assert_eq!(action.args, json!({"text":"@ada "}));
        let closes = host
            .calls()
            .into_iter()
            .filter(|call| matches!(call, testing::HostCall::Signal { payload, .. } if payload.is_null()))
            .count();
        assert_eq!(closes, 2);
    }

    #[tokio::test]
    async fn another_handlers_press_is_not_ours() {
        let host = testing::TestHost::default();
        let outcome = Handler(Default::default())
            .run(
                &json!({"handler":"tasks.sheet"}).to_string(),
                &context(&host),
            )
            .await
            .unwrap();
        assert_eq!(outcome, CommandOutcome::Handled { handled: false });
        assert!(host.calls().is_empty());
    }
}
