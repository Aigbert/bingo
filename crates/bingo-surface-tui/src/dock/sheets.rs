//! Sheet bodies are measured before the stream receives its remaining rows.
use crate::{
    clock::Now,
    layout::Layout,
    pane,
    tree::Tree,
    ui::{Open, Ui},
};
use bingo_sdk::slot::{Place, Shown};
use ratatui::{layout::Rect, text::Line};

pub fn prepare(tree: &Tree, ui: &Ui, frame: &mut Layout, now: Now) -> Option<Vec<Line<'static>>> {
    let area = frame.area();
    let cap = crate::layout::grid::dock_cap(usize::from(area.height));
    crate::ask::sheet(tree, ui, frame, now)
        .or_else(|| native(tree, ui, frame, cap, now))
        .or_else(|| crate::sign_in::prepare(tree, ui, frame, cap, now))
        .or_else(|| pane(tree, ui, frame, cap, now))
}
fn native(
    tree: &Tree,
    ui: &Ui,
    frame: &mut Layout,
    cap: u16,
    now: Now,
) -> Option<Vec<Line<'static>>> {
    let area = frame.area();
    if !ui.focus.showing() {
        return None;
    }
    let view = match ui.focus.open() {
        Open::Result { .. } => return super::result::draw(ui, frame, area.width, cap, now),
        Open::Rewind(card) => crate::rewind::sheet(
            &bingo_sdk::transcript::turns(&tree.viewed().transcript),
            card,
            area.width,
            cap,
        ),
        Open::Mode(selected) => {
            crate::mode::sheet::sheet(tree.viewed(), selected.as_deref(), area.width, cap)
        }
        Open::Keys(selected) => crate::keys::sheet::sheet(selected.clone(), area.width, cap),
        Open::Sessions(_) => {
            return Some(crate::sessions::draw(ui, tree, frame, area.width, cap, now));
        }
        Open::Plugins(_) => {
            return Some(crate::plugins::draw(ui, tree, frame, area.width, cap, now));
        }
        _ => return None,
    };
    Some(render_native(frame, view, area.width, now))
}

fn pane(
    tree: &Tree,
    ui: &Ui,
    frame: &mut Layout,
    cap: u16,
    now: Now,
) -> Option<Vec<Line<'static>>> {
    let area = frame.area();
    let state = tree.viewed();
    let chosen = pane::seating(ui, state, area)
        .panes
        .into_iter()
        .find(|seat| seat.docked())?;
    let pane = pane::wanted(ui, state)
        .into_iter()
        .find(|pane| pane.key == chosen.key)?;
    if !matches!(&ui.focus, crate::focus::Focus::Pane(key) if key == &pane.key) {
        return None;
    }
    let seat = pane::seat::Seated {
        body_rows: Some(cap),
        key: pane.key.clone(),
        place: Place::Dock,
        area: Rect::new(0, 0, area.width, cap),
    };
    let shown = pane::shown(ui, &pane, &seat);
    if crate::component::pane::matched(state, &shown)
        && let Shown::Pull { request } = &shown
    {
        frame
            .pulls
            .push(crate::pulled::Key::seated(state, request, &seat));
    }
    if let Some(view) = pane::view(ui, state, &pane, &seat) {
        targets(frame, view, area.width);
    }
    Some(frame.collect(pane::draw::lines(ui, state, &pane, &seat, now)))
}

fn inner(area: Rect) -> Rect {
    Rect {
        y: area.y.saturating_add(1),
        height: area.height.saturating_sub(2),
        ..area
    }
}

pub fn seat(tree: &Tree, ui: &Ui, frame: &mut Layout) {
    frame.seating.panes.retain(|seat| !seat.docked());
    if let Some(chosen) = pane::seating(ui, tree.viewed(), frame.area())
        .panes
        .into_iter()
        .find(|seat| seat.docked())
    {
        if !matches!(&ui.focus, crate::focus::Focus::Pane(key) if key == &chosen.key) {
            return;
        }
        let dock = frame.regions.composer;
        frame.seating.panes.push(pane::seat::Seated {
            body_rows: Some(crate::layout::grid::dock_cap(usize::from(
                frame.area().height,
            ))),
            key: chosen.key,
            place: Place::Dock,
            area: inner(dock),
        });
    }
}

pub fn draw(frame: &mut Layout, area: Rect, lines: Vec<Line<'static>>) {
    let skip = lines
        .len()
        .saturating_sub(usize::from(area.height.saturating_sub(2)));
    let inner = inner(area);
    frame.sheet_actions.retain_mut(|target| {
        if usize::from(target.area.y) < skip {
            return false;
        }
        target.area.y = target.area.y.saturating_sub(skip as u16) + inner.y;
        target.area.x += inner.x;
        inner.contains(target.area.as_position())
    });
    frame.render_widget(
        ratatui::widgets::Paragraph::new("").style(crate::dock::ground()),
        area,
    );
    frame.render_widget(
        ratatui::widgets::Paragraph::new(lines.into_iter().skip(skip).collect::<Vec<_>>()),
        inner,
    );
}

fn render_native(
    frame: &mut Layout,
    mut view: bingo_sdk::View,
    width: u16,
    now: Now,
) -> Vec<Line<'static>> {
    if let bingo_sdk::View::Box { layout, .. } = &mut view {
        layout.on_key = Some(bingo_sdk::Action {
            name: "surface:sheet".into(),
            args: serde_json::json!({}),
        });
    }
    let view = bingo_sdk::sheet::clicks(view);
    targets(frame, &view, width);
    frame.collect(crate::views::marked(
        &view,
        width.into(),
        &crate::views::Marks::at(now),
    ))
}

pub fn clicked(
    ui: &mut Ui,
    tree: &Tree,
    action: &bingo_sdk::Action,
    now: Now,
) -> Vec<crate::effect::Effect> {
    let Some(bingo_sdk::sheet::Press::Pick(id)) = bingo_sdk::sheet::pressed(&action.args) else {
        return vec![];
    };
    match ui.focus.open_mut() {
        Some(Open::Mode(selected)) => *selected = Some(id),
        Some(Open::Rewind(card)) => card.selected = Some(bingo_sdk::TurnId::from_raw(id)),
        _ => return vec![],
    }
    match ui.focus.open() {
        Open::Mode(_) => crate::mode::sheet::select(ui, tree, now),
        Open::Rewind(_) => crate::input::select_rewind(ui, tree, now),
        _ => vec![],
    }
}

pub fn targets(frame: &mut Layout, view: &bingo_sdk::View, width: u16) {
    let (mask, actions) = crate::views::targets::render(view, width.into());
    frame.sheet_actions =
        crate::views::targets::cells(&mask, &actions, Rect::new(0, 0, width, mask.len() as u16));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use bingo_sdk::{
        pick,
        sheet::Sheet,
        slot::{self, Slot},
    };
    #[test]
    fn a_click_chooses_the_visible_rewind_row_including_its_empty_cells() {
        let (mut state, _) = crate::test_support::ember::story::scene("done");
        let (mut ui, now) = scene();
        let now = still(now);
        ui.focus.show(Open::Rewind(Default::default()), now.instant);
        draw_sized(100, 32, &state, &mut ui, now);
        let target = ui
            .shown
            .sheet_actions
            .iter()
            .find(|target| {
                target.area.x == 70 && target.item.action.args["value"]["pick"] == "ember-0"
            })
            .unwrap();
        let mouse = crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: target.area.x,
            row: target.area.y,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        let effects = crate::pointer::on_mouse(&mut ui, &solo(&state), mouse, now);
        assert!(
            matches!(&effects[..], [crate::effect::Effect::Submit(bingo_sdk::Input::Text { text,.. })] if text == "/rewind ember-0")
        );
        state.apply(&frame(
            state.seq.0 + 1,
            bingo_sdk::Event::InteractionOpened {
                interaction: question(false, false),
            },
        ));
        ui.focus = crate::focus::Focus::Composer;
        assert!(
            !crate::pointer::on_mouse(&mut ui, &solo(&state), mouse, now)
                .iter()
                .any(|effect| matches!(
                    effect,
                    crate::effect::Effect::Submit(bingo_sdk::Input::Text { .. })
                ))
        );
    }
    #[test]
    fn a_dock_sheet_takes_stream_rows_and_preserves_the_draft() {
        let (mut ui, now) = scene();
        let now = still(now);
        ui.composer.set("keep this draft");
        let before = draw_sized(100, 32, &state(), &mut ui, now);
        let before_rows = ui.shown.regions.transcript.height;
        let view = Sheet {
            head: "Example".into(),
            fact: None,
            top: vec![],
            rows: (0..12)
                .map(|i| pick::Row::new(i.to_string(), pick::text(format!("row {i}"), "fg")))
                .collect(),
            selected: None,
            foot: vec![],
            pick: true,
        }
        .view(100, 19, Default::default());
        let state = folded(vec![frame(
            1,
            signalled(
                "example",
                "sheet",
                slot::with(
                    serde_json::json!({}),
                    [Slot::Pane {
                        hints: None,
                        id: "sheet".into(),
                        title: String::new(),
                        focus: true,
                        close_on_escape: true,
                        hold_toasts: false,
                        asked: true,
                        place: vec![Place::Dock],
                        size: slot::PaneSize::from_rows(12),
                        view: view.into(),
                    }],
                ),
            ),
        )]);
        ui.sync_panes(&state);
        let after = draw_sized(100, 32, &state, &mut ui, now);
        assert_ne!(before, after);
        assert!(ui.shown.regions.transcript.height < before_rows);
        assert_eq!(ui.composer.text(), "keep this draft");
        assert_eq!(
            ui.shown.regions.transcript.bottom(),
            ui.shown.regions.composer.y
        );
        assert_eq!(ui.shown.seating.panes[0].place, Place::Dock);
    }
}
