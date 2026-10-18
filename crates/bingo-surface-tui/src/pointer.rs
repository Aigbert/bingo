//! One pure function from a mouse event to a list of effects, against the
//! frame the last draw left behind.
//!
//! The pointer is the other half of [`crate::input`]: a key is a gesture with
//! one meaning, and a click is a gesture with a place — so where it landed is
//! the whole of this module's work, and what to do there is the same handful
//! of answers the keys give (§7: a key means one direction, a click means
//! both).

use bingo_sdk::{Level, SessionState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Position;

use crate::clock::Now;
use crate::effect::Effect;
use crate::fold;
use crate::input::{self, item_of, scroll};
use crate::select::{Cell, Edge};
use crate::tree::Tree;
use crate::ui::Ui;

/// Lines one notch of the wheel moves the transcript.
pub const WHEEL: isize = 3;

/// One pure function from a mouse event to a list of effects, against the
/// frame the last draw left behind: the wheel scrolls, a drag takes a run of
/// cells and the release copies it, a click lands on a block, on a child's
/// row, or on a card's option.
pub fn on_mouse(ui: &mut Ui, tree: &Tree, mouse: MouseEvent, now: Now) -> Vec<Effect> {
    match crate::focus::owner(ui, tree) {
        crate::focus::Owner::Search => return Vec::new(),
        crate::focus::Owner::Layer => return Vec::new(),
        crate::focus::Owner::Interaction => {
            return if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
                if let Some(target) = ui
                    .shown
                    .card_actions
                    .iter()
                    .find(|target| target.area.contains(Position::new(mouse.column, mouse.row)))
                {
                    return vec![Effect::Submit(crate::pulled(target.item.action.clone()))];
                }
                card_option(ui, mouse)
                    .map(|index| answer(ui, tree, index, now))
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
        }
        _ => {}
    }
    if let Some(effects) = crate::pane::mouse(ui, tree.viewed(), mouse, now) {
        return effects;
    }
    match mouse.kind {
        MouseEventKind::ScrollUp => scroll(ui, WHEEL, now),
        MouseEventKind::ScrollDown => scroll(ui, -WHEEL, now),
        MouseEventKind::Down(MouseButton::Left) => return pressed(ui, tree, mouse, now),
        MouseEventKind::Drag(MouseButton::Left) => drag(ui, mouse, now),
        MouseEventKind::Up(MouseButton::Left) => return released(ui, now),
        // The pointer is moving with nothing held down, so no hand is out past
        // the edge asking the view to walk any more.
        MouseEventKind::Moved => ui.select.dragging = None,
        _ => {}
    }
    Vec::new()
}

/// A press lands on whatever is under it: a card's option answers, a block
/// takes the focus, opens what is folded under it and starts a run.
fn pressed(ui: &mut Ui, tree: &Tree, mouse: MouseEvent, now: Now) -> Vec<Effect> {
    if matches!(ui.focus, crate::focus::Focus::Pane(_)) {
        ui.focus = crate::focus::Focus::Composer;
    }
    if let Some(effects) = crate::component::band::press(ui, Position::new(mouse.column, mouse.row))
    {
        return effects;
    }
    if ui
        .shown
        .crew
        .is_some_and(|area| area.contains(Position::new(mouse.column, mouse.row)))
    {
        return crate::crew::open(ui, tree, now);
    }
    // The way back to the foot answers a click as `end` answers a key.
    if on_foot(ui, mouse) {
        ui.scroll.end();
        return Vec::new();
    }
    // A picture is a thing to click before it is part of a block: a click on
    // one opens it, and a click beside it is the block's (§7).
    if let Some(picture) = picture(ui, mouse) {
        return vec![Effect::OpenPicture(picture.source)];
    }
    let Some(cell) = transcript_cell(ui, mouse) else {
        return Vec::new();
    };
    let block = ui.shown.blocks.at(cell.line);
    // A row that spawned a session is that session's row: `⏎` steps in, and
    // so does a click.
    if let Some(session) = block.as_ref().and_then(|item| tree.spawned_by(item)) {
        return vec![Effect::View(session.clone())];
    }
    if let Some(item) = &block {
        cycle_fold(ui, tree.viewed(), item, cell.line);
    }
    ui.select.block = block;
    ui.start_selection(cell);
    Vec::new()
}

/// A click on a block walks its fold one step and comes back round to the peek
/// its kind starts at (§7). A *key* never means two directions (§7, M11e); a
/// click on the same row is one gesture, and one gesture may go round.
///
/// A result, a notice and an action have the two states they always had — their
/// five-row cut and the whole of it. A thought that is over has three, and the
/// third is on the far side of the whole: the two rows it ended on, all of it,
/// then the row alone. Only a thought is worth putting away, and putting it
/// away is asked for rather than arrived at.
///
/// A turn opens and shuts from its folded line alone: a click on what an
/// opened turn shows under it is where a person starts selecting.
///
/// It writes the map `ctrl+o` writes, so a block is open in one way only.
fn cycle_fold(ui: &mut Ui, state: &SessionState, id: &bingo_sdk::ItemId, at: usize) {
    if let Some(holder) = ui.shown.blocks.roles().line_of(state, id) {
        let on_the_line = ui.shown.blocks.span(&holder).map(|(top, _)| top) == Some(at);
        if on_the_line {
            let next = match ui.folds.get(&holder) {
                Some(fold::Fold::Open) => fold::Fold::Peek,
                _ => fold::Fold::Open,
            };
            ui.folds.insert(holder, next);
        }
        return;
    }
    let Some(item) = item_of(state, id) else {
        return;
    };
    let next = fold::cycled(item, fold::fold_of(&ui.folds, item));
    ui.folds.insert(id.clone(), next);
}

/// A drag takes the far end of the run with it. Past the transcript's own
/// rows it takes the view too: the pointer is still pointing at the
/// transcript, and what it points at is off the screen (§3).
fn drag(ui: &mut Ui, mouse: MouseEvent, now: Now) {
    match edge_under(ui, mouse) {
        Some(edge) => drag_past(ui, edge, mouse, now),
        None => drag_inside(ui, mouse),
    }
}

/// On the region, the run reaches the cell under the pointer and the hand is
/// no longer asking the view for anything.
fn drag_inside(ui: &mut Ui, mouse: MouseEvent) {
    ui.select.dragging = None;
    if let Some(cell) = transcript_cell(ui, mouse) {
        ui.extend_selection(cell);
    }
}

/// Past it, only a run being drawn asks for anything: a drag that started
/// somewhere else is somebody else's gesture.
fn drag_past(ui: &mut Ui, edge: Edge, mouse: MouseEvent, now: Now) {
    if ui.focus.selection().is_none() {
        return;
    }
    ui.drag_edge(edge, column_under(ui, mouse), now);
}

/// The button comes up. A run that reaches somewhere is what the hand drew: it
/// goes to the terminal's clipboard by the road the keys use and is let go,
/// and a notice says how much, because a release is not obviously a copy. A
/// run that reaches nowhere is a click, and keeps every meaning a click has.
fn released(ui: &mut Ui, now: Now) -> Vec<Effect> {
    ui.select.dragging = None;
    let Some(lines) = ui
        .focus
        .selection()
        .filter(|run| !run.empty())
        .map(|run| run.lines())
    else {
        return Vec::new();
    };
    let copied = input::copy(ui);
    if !copied.is_empty() {
        ui.notify(Level::Info, taken(lines), now.instant);
    }
    copied
}

/// What a copy says it took.
fn taken(lines: usize) -> String {
    match lines {
        1 => "copied 1 line".to_string(),
        many => format!("copied {many} lines"),
    }
}

/// Which side of the transcript the pointer is pulling the view towards, if
/// either. Only the rows count: a row of the composer, of the status line or
/// of a placed view is a row below the transcript, whatever column it is in.
///
/// **The transcript's own first row is the top edge.** Nothing sits above it
/// (`frame`), so a hand that has gone past the top of the window is reported
/// on that row and on no other — there is no row above row 0 for a terminal
/// to name. It pulls by nothing at all, so the view does not jump under a
/// drag that merely reached the top; a hand that stays there is what walks it
/// (`Ui::drag_step`).
fn edge_under(ui: &Ui, mouse: MouseEvent) -> Option<Edge> {
    let region = ui.shown.regions.transcript;
    match i32::from(mouse.row) - i32::from(region.y) {
        0 => Some(Edge::Above(0)),
        row => Edge::of(row, usize::from(region.height)),
    }
}

/// The column of the transcript the pointer is over, whatever row it is on.
fn column_under(ui: &Ui, mouse: MouseEvent) -> usize {
    let region = ui.shown.regions.transcript;
    usize::from(mouse.column.saturating_sub(region.x))
}

/// The picture under the pointer, wherever this frame drew one — among the
/// transcript's rows, or on the composer's strip. A layer over the frame has
/// covered both, so while one is up the click is the layer's and no picture is
/// under it.
fn picture(ui: &Ui, mouse: MouseEvent) -> Option<crate::graphics::Picture> {
    if ui.focus.showing() {
        return None;
    }
    ui.shown.picture_at(Position {
        x: mouse.column,
        y: mouse.row,
    })
}

/// Whether the pointer is on the pill that leads back to the foot, where this
/// frame drew one. A layer over the frame has covered it, so while one is up
/// the click is the layer's.
fn on_foot(ui: &Ui, mouse: MouseEvent) -> bool {
    if ui.focus.showing() {
        return false;
    }
    ui.shown.foot.is_some_and(|area| {
        area.contains(Position {
            x: mouse.column,
            y: mouse.row,
        })
    })
}

/// Which option of the open card the pointer is on.
fn card_option(ui: &Ui, mouse: MouseEvent) -> Option<usize> {
    let painted = &ui.shown;
    let card = painted.card.as_ref()?;
    if !card.sealed_options.is_empty() {
        return card
            .sealed_options
            .iter()
            .find(|(area, _)| area.contains(Position::new(mouse.column, mouse.row)))
            .map(|(_, option)| *option);
    }
    let inside = card.area.contains(Position {
        x: mouse.column,
        y: mouse.row,
    });
    let row = usize::from(mouse.row.checked_sub(card.area.y + 1)?);
    inside
        .then(|| card.options.get(row).copied().flatten())
        .flatten()
}

/// Answer the open interaction on the option a click landed on.
fn answer(ui: &mut Ui, tree: &Tree, index: usize, now: Now) -> Vec<Effect> {
    let Some((_, interaction)) = tree.open_interaction() else {
        return Vec::new();
    };
    ui.dialog.focus = index;
    ui.dialog.on_key(interaction, mouse_enter(), now)
}

/// What a click on a card row means: the row it landed on, chosen.
fn mouse_enter() -> KeyEvent {
    KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
}

/// The transcript cell under the pointer, when it is over the transcript.
fn transcript_cell(ui: &Ui, mouse: MouseEvent) -> Option<Cell> {
    let painted = &ui.shown;
    let region = painted.regions.transcript;
    if !region.contains(Position {
        x: mouse.column,
        y: mouse.row,
    }) {
        return None;
    }
    Some(Cell {
        line: painted.line_at(usize::from(mouse.row - region.y))?,
        column: usize::from(mouse.column - region.x),
    })
}
