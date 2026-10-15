use bingo_sdk::{component::ModelPickerProps, *};
use serde_json::json;
#[derive(Clone, Default)]
pub struct Picker {
    pub id: String,
    pub props: ModelPickerProps,
    pub all: bool,
    pub sheet: Option<crate::sheet::Sheet>,
    pub message: Option<String>,
}
impl Picker {
    pub fn keep(&self) -> bool {
        self.props.scope == component::ModelScope::Keep
    }
    fn props(&self) -> ModelPickerProps {
        ModelPickerProps {
            sheet: self.sheet.as_ref().map(|sheet| sheet.title()),
            ..self.props.clone()
        }
    }
}
pub async fn publish(
    host: &HostHandle,
    session: &SessionId,
    picker: &Picker,
) -> Result<(), KernelError> {
    let request = render::PullRequest {
        component: render::Component::ModelPicker,
        request_id: picker.id.clone(),
        props: serde_json::to_value(picker.props()).map_err(crate::error)?,
        viewport: render::Viewport {
            place: None,
            columns: 108,
            rows: 18,
        },
    };
    let mut size = slot::PaneSize::from_rows(18);
    size.columns.ideal = slot::Length::Percent(100);
    let pane = slot::Slot::Pane {
        id: picker.id.clone(),
        title: "model".into(),
        focus: true,
        close_on_escape: true,
        hold_toasts: true,
        asked: true,
        place: placement(),
        size,
        view: slot::Shown::Pull { request },
    };
    host.signal(
        session,
        "bingo.mod.models",
        "picker",
        slot::with(json!({}), [pane]),
    )
    .await?;
    host.invalidate(session, vec![picker.id.clone()]).await
}
pub async fn close(host: &HostHandle, session: &SessionId) -> Result<(), KernelError> {
    host.signal(
        session,
        "bingo.mod.models",
        "picker",
        serde_json::Value::Null,
    )
    .await
}
pub fn chat(state: &SessionState) -> crate::card::Chat {
    crate::card::Chat {
        current: format!("{}/{}",state.summary.provider.as_deref().unwrap_or_default(),state.summary.model.as_deref().unwrap_or_default()),
        used: state.context.map_or(0,|context|context.used),
        pictures: state.transcript.model_items().any(|item| matches!(&item.body,ItemBody::User {parts,..} if parts.iter().any(|part|matches!(part,ContentPart::Image(_))))),
    }
}

pub fn placement() -> Vec<slot::Place> {
    vec![slot::Place::Rect(slot::Rect {
        anchor: slot::Anchor::BottomLeft,
        x: slot::Length::Cells(0),
        y: slot::Length::Cells(0),
        width: slot::Length::Percent(100),
        height: slot::Length::Cells(18),
    })]
}
