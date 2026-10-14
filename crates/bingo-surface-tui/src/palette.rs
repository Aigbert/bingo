//! Client placement and the actions minted for the open palette.
use crate::{
    clock::Now,
    effect::Effect,
    tree::Tree,
    ui::{Open, Ui},
};
use bingo_sdk::{
    component::{PaletteOpen, PaletteProps, Props},
    *,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::text::Line;
use serde_json::json;

#[derive(Clone, Debug, PartialEq)]
pub struct Opened {
    pub first: PaletteOpen,
    pub id: String,
    pub draft: String,
    pub sessions: Vec<SessionSummary>,
}
pub fn open(ui: &mut Ui, first: PaletteOpen, now: Now) -> Vec<Effect> {
    let draft = ui.composer.take();
    ui.composer.set(if first == PaletteOpen::Commands {
        "/"
    } else {
        ""
    });
    ui.focus.show(
        Open::Palette(Opened {
            first,
            id: IntentId::mint().to_string(),
            draft,
            sessions: Vec::new(),
        }),
        now.instant,
    );
    ui.edited();
    vec![Effect::ListStored]
}
pub fn close(ui: &mut Ui, now: Now) {
    if let Open::Palette(open) = ui.focus.open() {
        let draft = open.draft.clone();
        ui.composer.set(&draft);
    }
    ui.focus.close(now.instant);
    ui.edited();
}
pub fn active(ui: &Ui) -> bool {
    ui.focus.showing() && matches!(ui.focus.open(), Open::Palette(_))
}
pub fn request(ui: &Ui) -> Option<&str> {
    if let Open::Palette(open) = ui.focus.open() {
        Some(&open.id)
    } else {
        None
    }
}
fn props(ui: &Ui, state: &SessionState, open: &Opened) -> PaletteProps {
    let commands = ui
        .catalogs
        .commands
        .iter()
        .cloned()
        .chain(crate::commands::local_specs())
        .map(|spec| CatalogEntry {
            id: spec.name.clone(),
            label: spec.hint.clone(),
            meta: serde_json::to_value(spec).unwrap_or_default(),
        })
        .collect();
    let catalogs = ui.catalogs.values();
    PaletteProps {
        key_hints: crate::keys::BINDINGS
            .iter()
            .map(|binding| {
                (
                    binding.keys.into(),
                    format!("{} · {}", binding.group, binding.description),
                )
            })
            .collect(),
        query: ui.composer.text().into(),
        open: open.first,
        commands,
        bindings: crate::keys::commands::bindings(ui, state).commands,
        sessions: open.sessions.clone(),
        summary: state.summary.clone(),
        config: state.config.clone(),
        context: state.context,
        signals: state.signals.clone(),
        action: Action {
            name: "surface:palette".into(),
            args: json!({"handler":open.id}),
        },
        catalogs,
    }
}
pub fn draw(
    ui: &Ui,
    state: &SessionState,
    frame: &mut crate::layout::Layout,
    width: u16,
    rows: u16,
) -> Vec<Line<'static>> {
    let Open::Palette(open) = ui.focus.open() else {
        return Vec::new();
    };
    let props = Props::Palette(Box::new(props(ui, state, open)));
    let key = crate::component::key(state, &open.id, &props, width, rows);
    let answer = key.as_ref().and_then(|key| ui.placed.pulled.get(key));
    if let Some(key) = key {
        frame.pulls.push(key);
    }
    crate::component::draw(&props, answer, usize::from(width), fallback)
}
fn fallback(props: &Props, _: usize) -> Vec<Line<'static>> {
    let Props::Palette(props) = props else {
        return Vec::new();
    };
    let query = props.query.strip_prefix('/').unwrap_or(&props.query);
    props
        .commands
        .iter()
        .filter(|entry| entry.meta["hidden"] != true && entry.id.starts_with(query))
        .take(10)
        .map(|entry| Line::from(format!("   /{}", entry.id)))
        .collect()
}
fn handler(ui: &Ui) -> Option<Action> {
    let id = request(ui)?;
    let key = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.request_id == id)?;
    if key.request.props["query"].as_str() != Some(ui.composer.text()) {
        return None;
    }
    let View::Box { layout, .. } = ui.placed.pulled.get(key)? else {
        return None;
    };
    layout.on_key.clone()
}
pub fn waiting(ui: &Ui, state: &SessionState) -> bool {
    if !active(ui)
        || !render::matched(&state.config)
            .0
            .contains_key(&render::Component::Palette)
    {
        return false;
    }
    let Some(key) = ui
        .shown
        .pulls
        .iter()
        .find(|key| Some(key.request.request_id.as_str()) == request(ui))
    else {
        return true;
    };
    key.request.props["query"].as_str() != Some(ui.composer.text())
        || ui.placed.pulled.get(key).is_none()
}
pub fn key(ui: &mut Ui, tree: &Tree, key: KeyEvent, now: Now) -> Vec<Effect> {
    if let Some(mut action) = handler(ui) {
        if action.args.is_object()
            && let Some(chord) = crate::keys::commands::chord(key)
        {
            action.args["value"] = json!({"key":chord});
            return vec![Effect::SiteKey { action, key }];
        }
        return Vec::new();
    }
    match key.code {
        KeyCode::Esc => {
            close(ui, now);
            Vec::new()
        }
        KeyCode::Enter => {
            ui.focus.close(now.instant);
            crate::input::palette_submit(ui, tree, now)
        }
        _ => crate::input::palette_edit(ui, tree, key, now),
    }
}
pub fn result(ui: &mut Ui, tree: &Tree, action: &Action, now: Now) -> Vec<Effect> {
    if action.name != "surface:palette" || action.args["handler"].as_str() != request(ui) {
        return Vec::new();
    }
    let value = &action.args["value"];
    if let Some(text) = value["edit"].as_str() {
        ui.composer.set(text);
        ui.edited();
    }
    if let Some(message) = value["message"].as_str() {
        ui.notify(Level::Info, message, now.instant);
    }
    if let Some(key) = value["key"].as_str().and_then(crate::keys::commands::event) {
        close(ui, now);
        return crate::input::on_key(ui, tree, key, now);
    }
    if value["close"] == true {
        close(ui, now);
    }
    if let Some(line) = value["run"].as_str() {
        close(ui, now);
        return crate::input::command_line(ui, tree, now, line.into());
    }
    if let Some(id) = value["resume"].as_str() {
        close(ui, now);
        return vec![Effect::Open(SessionSelector::ById {
            id: SessionId::from_raw(id),
        })];
    }
    Vec::new()
}

#[cfg(test)]
mod tests;
