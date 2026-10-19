use bingo_sdk::{component::*, slot::Shown};

pub fn props(
    pane: &crate::pane::Pane,
    ui: &crate::ui::Ui,
    seat: &crate::pane::seat::Seated,
    offset: usize,
) -> Props {
    let focused = matches!(&ui.focus, crate::focus::Focus::Pane(key) if key == &pane.key);
    Props::Pane(PaneProps {
        hints: pane.hints.clone(),
        title: pane.title.clone(),
        is_focused: focused,
        body_columns: seat.area.width,
        placement: match seat.place {
            bingo_sdk::slot::Place::Above => "above",
            bingo_sdk::slot::Place::Dock => "dock",
            bingo_sdk::slot::Place::Side => "side",
            bingo_sdk::slot::Place::Overlay => "overlay",
            bingo_sdk::slot::Place::Rect(_) => "rect",
        }
        .into(),
        scroll: SiteScroll {
            offset,
            body_rows: usize::from(seat.body_rows.unwrap_or_else(|| {
                seat.area
                    .height
                    .saturating_sub(crate::pane::heading_rows(pane, seat))
            })),
        },
        view: SiteView { agent_id: None },
    })
}
pub fn shown(
    pane: &crate::pane::Pane,
    ui: &crate::ui::Ui,
    seat: &crate::pane::seat::Seated,
    offset: usize,
) -> Shown {
    let mut shown = pane.view.clone();
    if let Shown::Pull { request } = &mut shown {
        let mut value = request.props.as_object().cloned().unwrap_or_default();
        if let serde_json::Value::Object(fields) = props(pane, ui, seat, offset).value() {
            value.extend(fields);
        }
        request.props = value.into();
    }
    shown
}

pub fn matched(state: &bingo_sdk::SessionState, shown: &Shown) -> bool {
    match shown {
        Shown::View(_) => true,
        Shown::Pull { request } => bingo_sdk::render::matched(&state.config).asks(
            request.component,
            &request.on(bingo_sdk::render::SurfaceName::Terminal),
        ),
    }
}
