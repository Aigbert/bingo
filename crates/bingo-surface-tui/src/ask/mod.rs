mod body;
pub mod dock;
pub mod guard;
pub mod keys;
pub mod options;
pub mod question;
pub mod receipts;
pub mod scope;
pub mod spec;

use crate::{clock::Now, layout::Layout, tree::Tree, ui::Ui};
use bingo_sdk::{Action, Interaction, InteractionKind, SessionState, View};
use crossterm::event::KeyEvent;
use ratatui::text::Line;

/// The open permission or confirmation, read in the dock's words.
pub fn open<'a>(tree: &'a Tree, ui: &Ui) -> Option<(&'a Interaction, spec::Ask)> {
    let (owner, interaction) = tree.open_interaction()?;
    let worker = (owner.summary.id != *tree.view()).then(|| crate::tree::name(owner));
    let step = match &interaction.kind {
        InteractionKind::Permission { tool, input, .. } => Some(crate::tool_rows::resolve(
            tool,
            input,
            &crate::tool_rows::Looks::in_ui(ui),
            crate::theme::bullet().is_ascii(),
        )),
        _ => None,
    };
    let reading = spec::Reading {
        worker: worker.as_deref().unwrap_or("bingo"),
        home: crate::paths::home(),
        verb: step.as_ref().map_or("", |step| step.verb.as_str()),
        target: step.as_ref().map_or("", |step| step.target.as_str()),
    };
    Some((interaction, spec::ask(interaction, &reading)?))
}

/// The dock's rows while a permission or a confirmation is open.
pub fn sheet(tree: &Tree, ui: &Ui, frame: &mut Layout, now: Now) -> Option<Vec<Line<'static>>> {
    if ui.focus.showing() {
        return None;
    }
    if let Some(lines) = question::sheet(tree, ui, frame, now) {
        return Some(lines);
    }
    let (interaction, ask) = open(tree, ui)?;
    let picking = dock::Picking {
        cursor: ui.dialog.focus,
        typing: ui.dialog.words.as_ref().map(|words| words.text()),
        opened: guard::armed(ui, interaction, now),
        keyed_at: ui.keyed_at,
    };
    let width = frame.area().width;
    let lines = frame.collect(dock::lines(&ask, &picking, width, now));
    frame.sheet_actions = rows(lines.len(), ask.options.len(), width);
    Some(lines)
}

/// Where each answer sits, so a click on it picks and gives it.
fn rows(lines: usize, options: usize, width: u16) -> Vec<crate::views::targets::Target> {
    let first = lines.saturating_sub(options);
    (0..options)
        .map(|index| crate::views::targets::Target {
            area: ratatui::layout::Rect::new(0, (first + index) as u16, width, 1),
            item: bingo_sdk::ActionItem {
                meaning: None,
                label: String::new(),
                action: Action {
                    name: CLICK.into(),
                    args: serde_json::json!({ "pick": index }),
                },
                key: None,
            },
        })
        .collect()
}

/// The action an answer's row carries.
pub const CLICK: &str = "surface:ask";

/// The answer a click landed on.
pub fn clicked(ui: &Ui, column: u16, row: u16) -> Option<usize> {
    ui.shown
        .sheet_actions
        .iter()
        .filter(|target| target.item.action.name == CLICK)
        .find(|target| {
            target
                .area
                .contains(ratatui::layout::Position::new(column, row))
        })
        .and_then(|target| target.item.action.args["pick"].as_u64())
        .and_then(|index| usize::try_from(index).ok())
}

pub fn handler(ui: &Ui, interaction: &Interaction, key: KeyEvent) -> Option<Action> {
    if !matches!(
        interaction.kind,
        bingo_sdk::InteractionKind::Question(_) | bingo_sdk::InteractionKind::Form { .. }
    ) {
        return None;
    }
    let request = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.request_id == interaction.id.as_str())?;
    let View::Box { layout, .. } = ui.placed.pulled.get(request)? else {
        return None;
    };
    let mut action = layout.on_key.clone()?;
    action.args.as_object_mut()?.insert(
        "value".into(),
        serde_json::json!({"key":crate::keys::commands::chord(key)?}),
    );
    Some(action)
}

pub fn waiting(ui: &Ui, owner: &SessionState, interaction: &Interaction) -> bool {
    let Some(props) = crate::component::question::props(interaction) else {
        return false;
    };
    let area = ui.shown.canvas.buffer.area;
    let Some(key) = crate::component::key(
        owner,
        interaction.id.as_str(),
        &props,
        area.width,
        area.height,
    ) else {
        return false;
    };
    let request = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.request_id == interaction.id.as_str())
        .unwrap_or(&key);
    ui.placed.pulled.get(request).is_none()
}

#[cfg(test)]
mod tests;
