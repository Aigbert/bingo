pub mod aside;
pub mod drawing;
pub mod options;
pub mod receipts;

use crate::ui::Ui;
use bingo_sdk::{Action, Interaction, SessionState, View};
use crossterm::event::{KeyCode, KeyEvent};

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

pub fn begin_change(
    ui: &mut Ui,
    owner: &SessionState,
    interaction: &Interaction,
    key: KeyEvent,
) -> bool {
    if ui.dialog.words.is_some() {
        return false;
    }
    let options = options::options(interaction);
    let selected = match key.code {
        KeyCode::Enter => Some(ui.dialog.focus),
        KeyCode::Char(c @ '1'..='9') => c.to_digit(10).map(|n| n as usize - 1),
        _ => None,
    };
    let Some(index) = selected else { return false };
    if options
        .get(index)
        .is_none_or(|opt| opt.choice != options::Choice::Change)
    {
        return false;
    }
    let Some(bingo_sdk::component::Props::Permission(props)) =
        crate::component::sealed::props(owner, interaction)
    else {
        return false;
    };
    let text = props.input["command"]
        .as_str()
        .or_else(|| props.input["new_string"].as_str());
    let Some(text) = text else { return false };
    let mut words = crate::composer::Composer::default();
    words.set(text);
    ui.dialog.focus = index;
    ui.dialog.words = Some(words);
    ui.dialog.changing = Some(props.input);
    true
}

#[cfg(test)]
mod tests;
