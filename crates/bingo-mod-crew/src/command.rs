//! `/crew`, and the sheet it opens in the dock. The sheet keeps only its
//! picked identity, carried in its own props.
use bingo_sdk::*;
use serde_json::{Value, json};
pub struct Open(pub crate::Shared);
#[async_trait::async_trait]
impl Command for Open {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            name: "crew".into(),
            aliases: vec![],
            hint: "teammates and rooms, by who needs you".into(),
            args: ArgSpec::None,
            instant: true,
            family: "terminal".into(),
            hidden: false,
            opens: true,
            keys: vec![],
        }
    }
    async fn run(&self, _: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        open(
            &self.0,
            &cx.host,
            &cx.session,
            Some("Use /agents and /room for the plain tables.".into()),
        )
        .await
    }
}

pub async fn open(
    shared: &crate::Shared,
    host: &HostHandle,
    session: &SessionId,
    fallback: Option<String>,
) -> Result<CommandOutcome, KernelError> {
    shared
        .lock()
        .map_err(crate::error)?
        .insert(session.clone(), Default::default());
    publish(host, session, None).await?;
    Ok(CommandOutcome::Surface {
        action: Action {
            name: "Pane".into(),
            args: json!({"plugin":crate::ID,"id":crate::REQUEST}),
        },
        fallback,
    })
}

/// Place the sheet in the dock, picking `selected`, and draw it again.
pub async fn publish(
    host: &HostHandle,
    session: &SessionId,
    selected: Option<String>,
) -> Result<(), KernelError> {
    host.signal(session, crate::ID, crate::REQUEST, slot(selected))
        .await?;
    host.invalidate(session, vec![crate::REQUEST.into()]).await
}

pub fn slot(selected: Option<String>) -> Value {
    slot::with(
        json!({}),
        [slot::Slot::Pane {
            hints: None,
            id: crate::REQUEST.into(),
            title: String::new(),
            focus: true,
            close_on_escape: true,
            hold_toasts: false,
            asked: true,
            place: vec![slot::Place::Dock],
            size: slot::PaneSize::from_rows(12),
            view: slot::Shown::Pull {
                request: render::PullRequest {
                    component: render::Component::Pane,
                    request_id: crate::REQUEST.into(),
                    props: json!({"owner":crate::ID,"selected":selected}),
                    viewport: render::Viewport {
                        glyphs: Default::default(),
                        place: None,
                        columns: 0,
                        rows: 0,
                    },
                },
            },
        }],
    )
}

pub async fn close(
    host: &HostHandle,
    session: &SessionId,
    shared: &crate::Shared,
) -> Result<(), KernelError> {
    shared.lock().map_err(crate::error)?.remove(session);
    host.signal(session, crate::ID, crate::REQUEST, Value::Null)
        .await
}
