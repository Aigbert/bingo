use crate::{ID, REQUEST, Selections, error};
use bingo_sdk::slot::{Length, PaneSize, Shown, Slot};
use bingo_sdk::*;
use serde_json::{Value, json};

pub struct Open(pub Selections);
#[async_trait::async_trait]
impl Interceptor for Open {
    fn id(&self) -> &str {
        "status.open"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"command":"status"}),
            ..LinkMatcher::at(catalogue::COMMAND_RUN)
        }
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        cx: &HookContext,
        _: Value,
        _: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        self.0
            .lock()
            .map_err(error)?
            .insert(cx.session.clone(), "session".into());
        cx.host.signal(&cx.session, ID, REQUEST, slot()).await?;
        serde_json::to_value(CommandOutcome::Applied { message: None }).map_err(error)
    }
}
pub fn slot() -> Value {
    let mut size = PaneSize::from_rows(14);
    size.rows.max = Length::Percent(50);
    size.columns.ideal = Length::Percent(100);
    slot::with(
        json!({}),
        [Slot::Pane {
            id: REQUEST.into(),
            title: String::new(),
            focus: true,
            close_on_escape: true,
            hold_toasts: true,
            asked: true,
            place: slot::above(),
            size,
            view: Shown::Pull {
                request: render::PullRequest {
                    component: render::Component::Status,
                    request_id: REQUEST.into(),
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
pub async fn close(
    host: &HostHandle,
    session: &SessionId,
    selections: &Selections,
) -> Result<(), KernelError> {
    selections.lock().map_err(error)?.remove(session);
    host.signal(session, ID, REQUEST, Value::Null).await
}
