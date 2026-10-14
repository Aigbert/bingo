use bingo_sdk::{component::*, slot::Shown};

pub fn props(pane: &crate::pane::Pane, focused: bool, columns: u16, rows: u16) -> Props {
    Props::Pane(PaneProps {
        title: pane.title.clone(),
        is_focused: focused,
        body_columns: columns,
        placement: "above".into(),
        scroll: SiteScroll {
            offset: 0,
            body_rows: usize::from(rows),
        },
        view: SiteView { agent_id: None },
    })
}
pub fn shown(pane: &crate::pane::Pane, focused: bool, columns: u16, rows: u16) -> Shown {
    let mut shown = pane.view.clone();
    if let Shown::Pull { request } = &mut shown {
        let mut value = request.props.as_object().cloned().unwrap_or_default();
        if let serde_json::Value::Object(fields) = props(pane, focused, columns, rows).value() {
            value.extend(fields);
        }
        request.props = value.into();
    }
    shown
}

pub fn matched(state: &bingo_sdk::SessionState, shown: &Shown) -> bool {
    match shown {
        Shown::View(_) => true,
        Shown::Pull { request } => {
            bingo_sdk::render::matched(&state.config).asks(request.component, request)
        }
    }
}
