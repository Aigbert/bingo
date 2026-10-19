//! Draw seats and the status affordances derived from this layout.
use super::{Pane, edited, placed, seat::Seated, view, waiting};
use crate::{clock::Now, focus::Focus, layout::Layout, theme, ui::Ui, views};
use bingo_sdk::{
    SessionState,
    slot::{Place, Shown},
};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
};
use unicode_width::UnicodeWidthStr;

pub fn draw(ui: &Ui, state: &SessionState, frame: &mut Layout, now: Now) {
    // The hit-test reads this same stacking order backwards.
    frame
        .seating
        .panes
        .sort_by_key(|seat| matches!(seat.place, Place::Overlay | Place::Rect(_)));
    let panes = placed(state);
    for seated in frame.seating.panes.clone() {
        if let Some(pane) = panes.iter().find(|pane| pane.key == seated.key) {
            draw_one(ui, state, frame, pane, &seated, now);
        }
    }
}
fn draw_one(
    ui: &Ui,
    state: &SessionState,
    frame: &mut Layout,
    pane: &Pane,
    seated: &Seated,
    now: Now,
) {
    if seated.docked() {
        return;
    }
    let area = seated.area;
    let heading = super::heading_rows(pane, seated);
    let focused = matches!(&ui.focus, Focus::Pane(key) if key == &pane.key);
    frame.render_widget(Clear, area);
    if heading > 0 {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(&pane.title, theme::bold()))),
            Rect::new(area.x, area.y, area.width, area.height.min(1)),
        );
    }
    if focused && heading > 0 {
        let label = super::moved::label(seated.place);
        let width = (label.width() as u16).min(area.width);
        frame.render_widget(
            Paragraph::new(Span::styled(label, theme::dim())),
            Rect::new(area.right() - width, area.y, width, area.height.min(1)),
        );
    }
    let shown = super::shown(ui, pane, seated);
    if crate::component::pane::matched(state, &shown)
        && let Shown::Pull { request } = &shown
    {
        frame
            .pulls
            .push(crate::pulled::Key::seated(state, request, seated));
    }
    let body = Rect::new(
        area.x,
        area.y.saturating_add(heading),
        area.width,
        area.height.saturating_sub(heading),
    );
    let drawing = lines(ui, state, pane, seated, now);
    let lines = frame.collect(drawing);
    let top = offset(ui, seated, lines.len(), heading);
    frame.render_widget(
        Paragraph::new(lines.into_iter().skip(top).collect::<Vec<_>>()),
        body,
    );
    if let Some((key, run)) = &ui.placed.panes.selection
        && key == &pane.key
    {
        crate::select::mark(frame, body, top, run);
    }
}
pub(super) fn offset(ui: &Ui, seat: &Seated, lines: usize, heading: u16) -> usize {
    if seat.docked() {
        return lines.saturating_sub(usize::from(seat.area.height));
    }
    ui.placed
        .panes
        .scroll
        .get(&seat.key)
        .copied()
        .unwrap_or_default()
        .min(lines.saturating_sub(usize::from(seat.area.height.saturating_sub(heading))))
}
pub(crate) fn lines(
    ui: &Ui,
    state: &SessionState,
    pane: &Pane,
    seated: &Seated,
    now: Now,
) -> views::Drawing {
    let Some(view) = view(ui, state, pane, seated) else {
        return if crate::component::pane::matched(state, &super::shown(ui, pane, seated)) {
            vec![Line::from(theme::ellipsis())].into()
        } else {
            views::Drawing::default()
        };
    };
    let mut view = view.clone();
    let index = ui
        .placed
        .panes
        .selected
        .get(&pane.key)
        .copied()
        .unwrap_or_default();
    let focused = matches!(&ui.focus, Focus::Pane(key) if key == &pane.key);
    if !seated.docked() {
        edited(
            &mut view,
            index,
            ui.placed.panes.editors.get(&(pane.key.clone(), index)),
            focused,
            &mut 0,
        );
    }
    let props = crate::component::pane::props(
        pane,
        ui,
        seated,
        ui.placed
            .panes
            .scroll
            .get(&pane.key)
            .copied()
            .unwrap_or_default(),
    );
    crate::component::draw_marked(
        &props,
        Some(&view),
        usize::from(seated.area.width),
        &views::Marks {
            pending: ui.actions.action().cloned(),
            ..views::Marks::at(now)
        },
        |_, _| Vec::new(),
    )
}

pub fn status(ui: &Ui, state: &SessionState, frame: &mut Layout, area: Rect) -> Rect {
    if ui.focus.search().is_some() {
        return area;
    }
    let panes = placed(state);
    let mut entries = frame
        .seating
        .collapsed
        .iter()
        .filter_map(|key| {
            panes.iter().find(|pane| &pane.key == key).map(|pane| {
                (
                    pane.key.clone(),
                    format!(
                        "{} ▸",
                        if pane.title.is_empty() {
                            &pane.id
                        } else {
                            &pane.title
                        }
                    ),
                )
            })
        })
        .collect::<Vec<_>>();
    let waiting = waiting(ui, state);
    if let Some(pane) = waiting.first() {
        let count = waiting.len();
        entries.push((
            pane.key.clone(),
            format!("{count} pane{} waiting", if count == 1 { "" } else { "s" }),
        ));
    }
    let mut right = area.right();
    for (key, text) in entries.into_iter().rev() {
        let text = views::clip(&text, usize::from(right.saturating_sub(area.x)));
        let width = text.width() as u16;
        let target = Rect::new(right - width, area.y, width, area.height);
        frame.waiting.push((key, target));
        frame.render_widget(Paragraph::new(Span::styled(text, theme::dim())), target);
        right = target.x.saturating_sub(2).max(area.x);
    }
    Rect::new(area.x, area.y, right.saturating_sub(area.x), area.height)
}
