use bingo_sdk::*;
use serde_json::{Value, json};
pub struct Open(pub crate::Shared);
#[async_trait::async_trait]
impl Command for Open {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            name: "crew".into(),
            aliases: vec![],
            hint: "agents, rooms and tasks in one panel".into(),
            args: ArgSpec::None,
            instant: true,
            family: "terminal".into(),
            hidden: false,
            opens: true,
            keys: vec!["ctrl+n".into()],
        }
    }
    async fn run(&self, _: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        self.0
            .lock()
            .map_err(crate::error)?
            .insert(cx.session.clone(), Default::default());
        cx.host
            .signal(&cx.session, crate::ID, crate::REQUEST, slot())
            .await?;
        cx.host
            .invalidate(&cx.session, vec![crate::REQUEST.into()])
            .await?;
        Ok(CommandOutcome::Surface {
            action: Action {
                name: "Pane".into(),
                args: json!({"plugin":crate::ID,"id":crate::REQUEST}),
            },
            fallback: Some("Use /agents, /rooms and /tasks for the plain tables.".into()),
        })
    }
}
pub fn slot() -> Value {
    let mut size = slot::PaneSize::from_rows(18);
    size.rows.max = slot::Length::Percent(80);
    size.columns.ideal = slot::Length::Percent(100);
    slot::with(
        json!({}),
        [slot::Slot::Pane {
            id: crate::REQUEST.into(),
            title: "crew".into(),
            focus: false,
            close_on_escape: false,
            hold_toasts: false,
            asked: true,
            place: slot::above(),
            size,
            view: slot::Shown::Pull {
                request: render::PullRequest {
                    component: render::Component::Pane,
                    request_id: crate::REQUEST.into(),
                    props: json!({}),
                    viewport: render::Viewport {
                        place: None,
                        columns: 0,
                        rows: 0,
                    },
                },
            },
        }],
    )
}
pub async fn close(cx: &CommandContext, shared: &crate::Shared) -> Result<(), KernelError> {
    shared.lock().map_err(crate::error)?.remove(&cx.session);
    cx.host
        .signal(&cx.session, crate::ID, crate::REQUEST, Value::Null)
        .await
}
