use crate::{focus::Focus, layout::Layout, ui::Ui};
use bingo_sdk::{
    SessionState, View,
    component::{KeyPair, SiteHints},
};

pub fn hints(state: &SessionState, ui: &Ui, frame: &Layout) -> Option<SiteHints> {
    if let Some(hints) = sheet_hints(ui) {
        return Some(hints);
    }
    if crate::mention_list::query(ui).is_some() {
        return Some(SiteHints::new(
            crate::mention_list::HINTS.map(|(key, action)| KeyPair::new(key, action)),
        ));
    }
    if matches!(ui.focus, Focus::Detail(_)) {
        return Some(SiteHints::new([
            KeyPair::new("↑↓", "step"),
            KeyPair::new("⏎", "open"),
            KeyPair::new("esc", "close"),
        ]));
    }
    if ui.focus.search().is_some() {
        return Some(search_hints());
    }
    if let Some(pager) = crate::browse::reader::current(ui)
        && ui.focus.showing()
    {
        return Some(pager.search.as_ref().map_or_else(
            || {
                SiteHints::new([
                    KeyPair::new("j/k", "scroll"),
                    KeyPair::new("pgup/pgdn", "page"),
                    KeyPair::new("g/G", "ends"),
                    KeyPair::new("/", "search"),
                    KeyPair::new("esc", "back"),
                ])
            },
            |_| search_hints(),
        ));
    }
    if let Focus::Pane(focused) = &ui.focus {
        let pane = crate::pane::placed(state)
            .into_iter()
            .find(|pane| &pane.key == focused)?;
        let seated = frame
            .seating
            .panes
            .iter()
            .find(|seat| &seat.key == focused)?;
        return crate::pane::view(ui, state, &pane, seated)
            .and_then(from_view)
            .cloned()
            .or(pane.hints);
    }
    let id = crate::palette::request(ui)
        .filter(|_| crate::palette::active(ui))
        .or_else(|| crate::plugins::request(ui).filter(|_| crate::plugins::active(ui)))
        .or_else(|| crate::sessions::request(ui).filter(|_| crate::sessions::active(ui)))
        .or_else(|| crate::browse::request(ui).filter(|_| crate::browse::active(ui)))?;
    frame
        .pulls
        .iter()
        .find(|key| key.request.request_id == id)
        .and_then(|key| ui.placed.pulled.display(key))
        .and_then(from_view)
        .cloned()
        .or_else(|| {
            ui.placed
                .pulled
                .drawings(&state.summary.id)
                .find(|(key, _)| key.request.request_id == id)
                .and_then(|(_, view)| from_view(view))
                .cloned()
        })
}
fn sheet_hints(ui: &Ui) -> Option<SiteHints> {
    if ui.focus.showing() && matches!(ui.focus.open(), crate::ui::Open::Keys(_)) {
        return Some(SiteHints::new([KeyPair::new("esc", "close")]));
    }
    if ui.focus.showing() {
        let pairs = match ui.focus.open() {
            crate::ui::Open::Mode(_) => Some(vec![
                KeyPair::new("↑↓", "mode"),
                KeyPair::new("⏎", "use"),
                KeyPair::new("esc", "close"),
            ]),
            crate::ui::Open::Rewind(_) => Some(vec![
                KeyPair::new("↑↓", "prompt"),
                KeyPair::new("⏎", "rewind"),
                KeyPair::new("esc", "close"),
            ]),
            crate::ui::Open::Result { .. } => Some(vec![
                KeyPair::new("↑↓", "scroll"),
                KeyPair::new("esc", "close"),
            ]),
            _ => None,
        };
        if let Some(pairs) = pairs {
            return Some(SiteHints::new(pairs));
        }
    }
    None
}
fn from_view(view: &View) -> Option<&SiteHints> {
    match view {
        View::Box { layout, .. } => layout.hints.as_ref(),
        _ => None,
    }
}

fn search_hints() -> SiteHints {
    SiteHints::new([
        KeyPair::new("↑↓", "matches"),
        KeyPair::new("⏎", "jump"),
        KeyPair::new("esc", "close"),
    ])
}
