//! A question from the model, drawn in the dock from the `ask` mod's pulled
//! view, so a mod can redraw it. The surface keeps the guard: each tab the
//! mod brings on screen holds keys back again while its picked row fades in.
use super::{dock, guard};
use crate::{
    clock::{self, Now},
    component::sealed::Site,
    layout::{Layout, grid},
    tree::{self, Tree},
    ui::Ui,
    views,
};
use bingo_sdk::{Interaction, InteractionKind, View, component::QuestionFocus};
use jiff::Timestamp;
use ratatui::text::Line;

pub fn sheet(tree: &Tree, ui: &Ui, frame: &mut Layout, now: Now) -> Option<Vec<Line<'static>>> {
    let (owner, interaction) = tree.open_interaction()?;
    if !asks(interaction) {
        return None;
    }
    let area = frame.area();
    let agent = (owner.summary.id != *tree.view()).then(|| tree::name(owner));
    let tools = crate::tool_rows::Looks::in_ui(ui);
    let site = Site {
        state: owner,
        tools: &tools,
        ui,
        interaction,
        agent: agent.as_deref(),
        width: usize::from(area.width),
        room: usize::from(grid::dock_cap(usize::from(area.height))) + 1,
        now,
    };
    let drawn = site.rows(&mut frame.pulls, false);
    let drawn = frame.collect(drawn);
    let opened = guard::armed(ui, interaction, now);
    let fade = guard::fade(opened, now);
    let (caret_now, rest) = dock::rested(now, ui.keyed_at);
    let writing = focus(ui, &frame.pulls, interaction).is_some_and(|focus| focus.writing);
    let lines = drawn
        .rows
        .into_iter()
        .map(|(line, _)| dock::lit(line, fade, writing, caret_now))
        .collect();
    let caret = (writing && caret_now.motion).then(|| (now.instant + clock::FRAME).min(rest));
    Some(frame.collect(views::Drawing {
        lines,
        next: guard::next(opened, now).into_iter().chain(caret).min(),
    }))
}

fn asks(interaction: &Interaction) -> bool {
    matches!(
        interaction.kind,
        InteractionKind::Question(_) | InteractionKind::Form { .. }
    )
}

/// When the ask's current tab came on screen: the ask's own opening until
/// the mod moves to another question.
pub fn since(ui: &Ui, interaction: &Interaction) -> Timestamp {
    ui.dialog
        .tab_at
        .filter(|_| asks(interaction))
        .map_or(interaction.opened_at, |(_, at)| at)
}

/// The mod's editing state for the open question, from this frame's pull.
pub fn focus<'a>(
    ui: &'a Ui,
    pulls: &[crate::pulled::Key],
    interaction: &Interaction,
) -> Option<&'a QuestionFocus> {
    let key = pulls
        .iter()
        .find(|key| key.request.request_id == interaction.id.as_str())?;
    let View::Box { layout, .. } = ui.placed.pulled.get(key)? else {
        return None;
    };
    layout.question.as_ref()
}

/// Whether the person is typing their own words into the open question.
pub fn writing(ui: &Ui, interaction: &Interaction) -> bool {
    focus(ui, &ui.shown.pulls, interaction).is_some_and(|focus| focus.writing)
}

/// A drawing of the open question came back: note the moment it shows
/// another tab.
pub fn shown(ui: &mut Ui, request: &str, view: &View, wall: Timestamp) {
    if ui.dialog.current().map(|id| id.as_str()) != Some(request) {
        return;
    }
    let View::Box { layout, .. } = view else {
        return;
    };
    let Some(focus) = &layout.question else {
        return;
    };
    let on = ui.dialog.tab_at.map_or(0, |(tab, _)| tab);
    if focus.field != on {
        ui.dialog.tab_at = Some((focus.field, wall));
    }
}
