//! The directory's placement and client actions; its mod owns the feature.
use crate::{
    clock::Now,
    effect::Effect,
    tree::Tree,
    ui::{Open, Ui},
};
use bingo_sdk::{
    component::{Props, SessionsProps, SessionsScope},
    *,
};
use crossterm::event::{KeyCode, KeyEvent};
use serde_json::json;
#[derive(Clone, Debug, PartialEq)]
pub struct Opened {
    pub id: String,
    pub draft: String,
    pub scope: SessionsScope,
    pub limit: usize,
    pub sessions: Vec<SessionSummary>,
}
pub fn open(ui: &mut Ui, now: Now) -> Vec<Effect> {
    let draft = ui.composer.take();
    ui.focus.show(
        Open::Sessions(Opened {
            id: IntentId::mint().to_string(),
            draft,
            scope: SessionsScope::Here,
            limit: 50,
            sessions: vec![],
        }),
        now.instant,
    );
    ui.edited();
    vec![Effect::ListSessions]
}
pub fn active(ui: &Ui) -> bool {
    ui.focus.showing() && matches!(ui.focus.open(), Open::Sessions(_))
}
pub fn request(ui: &Ui) -> Option<&str> {
    if let Open::Sessions(open) = ui.focus.open() {
        Some(&open.id)
    } else {
        None
    }
}
pub fn close(ui: &mut Ui, now: Now) {
    if let Open::Sessions(open) = ui.focus.open() {
        let draft = open.draft.clone();
        ui.composer.set(&draft);
    }
    ui.focus.close(now.instant);
    ui.dialog.aside = false;
    ui.edited();
}
fn props(ui: &Ui, tree: &Tree, now: Now) -> Option<SessionsProps> {
    let Open::Sessions(open) = ui.focus.open() else {
        return None;
    };
    Some(SessionsProps {
        session: tree.view().clone(),
        cwd: tree.viewed().summary.cwd.clone(),
        query: ui.composer.text().into(),
        scope: open.scope,
        limit: open.limit,
        sessions: open.sessions.clone(),
        now: jiff::Timestamp::from_second(now.wall.as_second() / 60 * 60).unwrap_or(now.wall),
        action: Action {
            name: "surface:sessions".into(),
            args: json!({"handler":open.id}),
        },
    })
}
pub fn draw(
    ui: &Ui,
    tree: &Tree,
    frame: &mut crate::layout::Layout,
    width: u16,
    height: u16,
    now: Now,
) -> Vec<ratatui::text::Line<'static>> {
    let Some(props) = props(ui, tree, now) else {
        return vec![];
    };
    let Some(id) = request(ui) else {
        return vec![];
    };
    let props = Props::Sessions(Box::new(props));
    let key = crate::component::key(tree.viewed(), id, &props, width, height);
    let answer = key.as_ref().and_then(|key| ui.placed.pulled.get(key));
    if let Some(key) = key {
        frame.pulls.push(key);
    }
    let drawing = crate::component::draw_marked(
        &props,
        answer,
        usize::from(width),
        &crate::views::Marks::at(now),
        |_, _| {
            vec![ratatui::text::Line::raw(
                " Sessions is off · esc to close · ctrl+r for recent sessions",
            )]
        },
    );
    frame.collect(drawing)
}
pub fn waiting(ui: &Ui, tree: &Tree, now: Now) -> bool {
    if !active(ui) {
        return false;
    }
    let Some(props) = props(ui, tree, now) else {
        return false;
    };
    let Some(id) = request(ui) else {
        return false;
    };
    let area = ui.shown.regions.above();
    let Some(key) = crate::component::key(
        tree.viewed(),
        id,
        &Props::Sessions(Box::new(props)),
        area.width,
        area.height,
    ) else {
        return false;
    };
    ui.placed.pulled.get(&key).is_none()
}
pub fn key(ui: &mut Ui, tree: &Tree, event: KeyEvent, now: Now) -> Vec<Effect> {
    if let Some(id) = request(ui)
        && let Some(key) = ui.shown.pulls.iter().find(|k| {
            k.request.request_id == id
                && k.request.props["query"].as_str() == Some(ui.composer.text())
        })
        && let Some(View::Box { layout, .. }) = ui.placed.pulled.get(key)
        && let Some(chord) = crate::keys::commands::chord(event)
        && let Some(action) = layout
            .on_key
            .clone()
            .and_then(|on_key| on_key.with_value(json!({"key":chord})))
    {
        return vec![Effect::SiteKey { action, key: event }];
    }
    if event.code == KeyCode::Esc {
        close(ui, now);
        return vec![];
    }
    crate::input::palette_edit(ui, tree, event, now)
}
pub fn result(ui: &mut Ui, action: &Action, now: Now) -> Vec<Effect> {
    if action.name != "surface:sessions" || action.args["handler"].as_str() != request(ui) {
        return vec![];
    }
    let value = &action.args["value"];
    if let Some(text) = value["edit"].as_str() {
        ui.composer.set(text);
        ui.edited();
    }
    if value["close"] == true {
        close(ui, now);
    }
    if let Some(id) = value["resume"].as_str() {
        close(ui, now);
        return vec![Effect::Open(SessionSelector::ById {
            id: SessionId::from_raw(id),
        })];
    }
    refresh(ui, value)
}
fn refresh(ui: &mut Ui, value: &serde_json::Value) -> Vec<Effect> {
    let Some(Open::Sessions(open)) = ui.focus.open_mut() else {
        return vec![];
    };
    let mut refresh = value["refresh"] == true;
    if let Ok(scope) = serde_json::from_value(value["scope"].clone()) {
        open.scope = scope;
        open.sessions.clear();
        refresh = true;
    }
    if value["more"] == true {
        open.limit += 50;
        refresh = true;
    }
    if refresh {
        vec![Effect::ListSessions]
    } else {
        vec![]
    }
}

pub fn exists(state: &bingo_sdk::SessionState) -> bool {
    bingo_sdk::render::matched(&state.config)
        .0
        .contains_key(&bingo_sdk::render::Component::Sessions)
}

#[cfg(test)]
mod tests;
