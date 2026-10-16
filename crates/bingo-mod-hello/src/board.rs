use bingo_sdk::{
    slot::{self, Slot},
    *,
};
use serde_json::{Value, json};

const ANCHOR: &str = "HELLO_BOARD";
pub struct BoardCommand(pub &'static str);
pub struct BoardRow;

pub fn board(state: &str) -> View {
    View::Panel {
        title: "Board".into(),
        child: Box::new(View::column(vec![
            table(state),
            View::Actions { items: actions() },
        ])),
    }
}

fn table(state: &str) -> View {
    View::Table {
        headers: vec!["id".into(), "step".into(), "state".into()],
        rows: vec![vec!["1".into(), "Fixture".into(), state.into()]],
    }
}

fn actions() -> Vec<ActionItem> {
    [("Tick", "hello.tick"), ("Reset", "hello.reset")]
        .into_iter()
        .map(|(label, name)| ActionItem {
            label: label.into(),
            action: Action {
                name: name.into(),
                args: Value::Null,
            },
            key: None,
        })
        .collect()
}

fn state(payload: &Value) -> &str {
    payload
        .pointer("/child/children/0/rows/0/2")
        .and_then(Value::as_str)
        .unwrap_or("pending")
}

fn payload(snapshot: &SessionState) -> Value {
    snapshot
        .extensions
        .get("hello")
        .and_then(|kinds| kinds.get("board"))
        .cloned()
        .unwrap_or(Value::Null)
}

fn anchor(snapshot: &SessionState) -> Option<ItemId> {
    snapshot
        .transcript
        .items()
        .iter()
        .rev()
        .find(|item| matches!(&item.body, ItemBody::Notice { code, .. } if code == ANCHOR))
        .map(|item| item.id.clone())
}

async fn publish(
    host: &HostHandle,
    session: &SessionId,
    state: &str,
    item: Option<ItemId>,
) -> Result<(), KernelError> {
    let mut attachment = host
        .open(
            SessionSelector::ById {
                id: session.clone(),
            },
            ClientIdentity {
                name: "hello".into(),
                surface: "hello".into(),
            },
            OpenOptions::default(),
        )
        .await?;
    let slots = item.map(|item| Slot::Row {
        item,
        mark: None,
        title: Some("Board".into()),
        badge: None,
        caption: None,
        body: Some(table(state)),
        actions: actions(),
    });
    host.extend(
        session,
        "hello",
        "board",
        slot::with(json!(board(state)), slots),
    )
    .await?;
    published(&mut attachment.events).await
}

#[async_trait::async_trait]
impl Command for BoardCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            opens: false,
            name: self.0.into(),
            keys: vec![],
            hint: "Exercise a durable fixture row".into(),
            aliases: vec![],
            args: ArgSpec::None,
            instant: true,
            hidden: false,
            family: "fixture".into(),
        }
    }
    async fn run(&self, _: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let _writer = super::writer::acquire().await;
        let snapshot = cx.host.read(&cx.session).await?;
        let value = payload(&snapshot);
        let state = match (self.0, state(&value)) {
            ("hello.reset", _) => "pending",
            ("hello.tick", "pending") => "running",
            ("hello.tick", _) => "done",
            (_, current) => current,
        };
        if self.0 == "hello.reset" {
            cx.host
                .signal(&cx.session, "hello", "progress", Value::Null)
                .await?;
        }
        let item = anchor(&snapshot);
        publish(&cx.host, &cx.session, state, item.clone()).await?;
        Ok(if item.is_none() {
            CommandOutcome::Record {
                body: Box::new(ItemBody::Notice {
                    level: Level::Info,
                    code: ANCHOR.into(),
                    text: "Board".into(),
                }),
            }
        } else {
            CommandOutcome::Applied {
                message: Some(
                    if self.0 == "hello.reset" {
                        "board reset"
                    } else {
                        "board published"
                    }
                    .into(),
                ),
            }
        })
    }
}

#[async_trait::async_trait]
impl Hook for BoardRow {
    fn id(&self) -> &str {
        "hello:board-row"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![HookPoint::Event],
            tool: None,
        }
    }
    async fn on_event(&self, frame: &Frame, cx: &HookContext) {
        let Event::ItemCompleted { item } = &frame.event else {
            return;
        };
        if !matches!(&item.body, ItemBody::Notice { code, .. } if code == ANCHOR) {
            return;
        }
        let result = async {
            let _writer = super::writer::acquire().await;
            let snapshot = cx.host.read(&frame.session).await?;
            publish(
                &cx.host,
                &frame.session,
                state(&payload(&snapshot)),
                Some(item.id.clone()),
            )
            .await
        }
        .await;
        if let Err(error) = result {
            let _ = cx
                .host
                .notice(Level::Warn, "HELLO_PLACEMENT_FAILED", &error.message)
                .await;
        }
    }
}

async fn published(events: &mut FrameStream) -> Result<(), KernelError> {
    while let Some(frame) = std::future::poll_fn(|cx| events.as_mut().poll_next(cx)).await {
        match frame.event {
            Event::Extension { plugin, kind, .. } if plugin == "hello" && kind == "board" => {
                return Ok(());
            }
            Event::SessionClosed { .. } => break,
            _ => {}
        }
    }
    Err(KernelError::new(
        ErrorCode::SessionClosed,
        "session closed before fixture publication",
    ))
}
