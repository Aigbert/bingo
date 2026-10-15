//! Placement and client projection of the Browse owner's drawing.
mod actions;
mod paint;
use crate::{
    clock::Now,
    effect::Effect,
    tree::Tree,
    ui::{Open, Ui},
};
pub use actions::{composer, result};
use bingo_sdk::{
    component::{BrowseProps, Props},
    *,
};
use crossterm::event::{KeyCode, KeyEvent};
pub use paint::{folds, mark, top};
use serde_json::json;
#[derive(Clone, Debug, PartialEq)]
pub struct Opened {
    pub session: SessionId,
    pub id: String,
    pub selected: Option<TurnId>,
    pub confirm: bool,
}
pub fn open(ui: &mut Ui, tree: &Tree, selected: Option<TurnId>, confirm: bool, now: Now) {
    if tree.viewed().summary.parent.is_some() {
        ui.notify(
            Level::Info,
            "Browse is available in the root conversation",
            now.instant,
        );
        return;
    }
    let selected = selected.or_else(|| {
        transcript::turns(&tree.viewed().transcript)
            .last()
            .map(|t| t.id.clone())
    });
    ui.focus.show(
        Open::Browse(Opened {
            session: tree.view().clone(),
            id: IntentId::mint().to_string(),
            selected,
            confirm,
        }),
        now.instant,
    );
}
pub fn active(ui: &Ui) -> bool {
    ui.focus.showing() && matches!(ui.focus.open(), Open::Browse(_))
}
pub fn request(ui: &Ui) -> Option<&str> {
    if let Open::Browse(open) = ui.focus.open() {
        Some(&open.id)
    } else {
        None
    }
}
fn answer(ui: &Ui) -> Option<&View> {
    let Open::Browse(open) = ui.focus.open() else {
        return None;
    };
    ui.placed
        .pulled
        .drawings(&open.session)
        .find(|(key, _)| key.request.request_id == open.id)
        .map(|(_, view)| view)
}
fn handler(ui: &Ui) -> Option<Action> {
    if let View::Box { layout, .. } = answer(ui)? {
        layout.on_key.clone()
    } else {
        None
    }
}
pub fn current(ui: &Ui) -> Option<BrowseProps> {
    serde_json::from_value(handler(ui)?.args["props"].clone()).ok()
}
fn props(ui: &Ui, tree: &Tree) -> Option<BrowseProps> {
    let Open::Browse(open) = ui.focus.open() else {
        return None;
    };
    let mut props = current(ui).unwrap_or_else(|| BrowseProps {
        session: tree.view().clone(),
        selected: open.selected.clone(),
        query: String::new(),
        matches: vec![],
        turns: vec![],
        current: 0,
        action: Action {
            name: "surface:browse".into(),
            args: json!({"handler":open.id}),
        },
        confirm: open.confirm,
    });
    props.turns = transcript::turns(&tree.viewed().transcript);
    props.matches = transcript::find(&tree.viewed().transcript, &props.query);
    Some(props)
}
pub fn offered(ui: &Ui, tree: &Tree) -> bool {
    tree.viewed().summary.parent.is_none()
        && bingo_sdk::render::matched(&tree.viewed().config)
            .0
            .contains_key(&render::Component::Browse)
        && ui.catalogs.commands.iter().any(|s| s.name == "browse")
}
pub fn draw(
    ui: &Ui,
    tree: &Tree,
    frame: &mut crate::layout::Layout,
    width: u16,
    rows: u16,
) -> Vec<ratatui::text::Line<'static>> {
    let Some(props) = props(ui, tree) else {
        return vec![];
    };
    let props = Props::Browse(Box::new(props));
    let key = crate::component::key(
        tree.viewed(),
        request(ui).unwrap_or_default(),
        &props,
        width,
        rows,
    );
    let answer = key.as_ref().and_then(|key| ui.placed.pulled.display(key));
    let lines = crate::component::draw(&props, answer, width.into(), |_, _| {
        vec![ratatui::text::Line::from(if offered(ui, tree) {
            "browse · loading · esc close"
        } else {
            "browse unavailable · ctrl+f line search · esc close"
        })]
    });
    if let Some(key) = key {
        frame.pulls.push(key);
    }
    lines
}
pub fn waiting(ui: &Ui, tree: &Tree) -> bool {
    if !active(ui) || !offered(ui, tree) {
        return false;
    }
    let Some(props) = props(ui, tree) else {
        return false;
    };
    let above = ui.shown.regions.above();
    if crate::component::key(
        tree.viewed(),
        request(ui).unwrap_or_default(),
        &Props::Browse(Box::new(props.clone())),
        above.width,
        above.height.min(ui.shown.canvas.area().height / 2),
    )
    .is_none()
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
    key.request.props != Props::Browse(Box::new(props)).value()
        || ui
            .placed
            .pulled
            .current(
                render::Component::Browse,
                request(ui).unwrap_or_default(),
                tree.viewed(),
            )
            .is_none_or(|(_, waiting)| waiting)
}
pub fn key(ui: &mut Ui, tree: &Tree, key: KeyEvent, now: Now) -> Vec<Effect> {
    if offered(ui, tree)
        && ui
            .shown
            .pulls
            .iter()
            .any(|key| Some(key.request.request_id.as_str()) == request(ui))
        && let Some(mut action) = handler(ui)
        && let Some(chord) = crate::keys::commands::chord(key)
    {
        action.args["value"] = json!({"key":chord});
        return vec![Effect::SiteKey { action, key }];
    }
    if key == KeyEvent::new(KeyCode::Char('f'), crossterm::event::KeyModifiers::CONTROL) {
        ui.focus = crate::focus::Focus::Search(crate::search::Search::open());
    } else if key.code == KeyCode::Esc {
        ui.focus.close(now.instant);
    }
    Vec::new()
}

#[cfg(test)]
mod tests;
