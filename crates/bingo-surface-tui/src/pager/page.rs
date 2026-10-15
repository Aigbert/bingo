//! The `Pager` client action: one typed request to open the sheet, whoever
//! asks — a command receipt, Browse's `enter`, the memory panel's `enter`.
use bingo_sdk::ItemId;
use serde::Deserialize;

use super::{Pager, Return, Source};
use crate::clock::Now;
use crate::focus::Focus;
use crate::tree::Tree;
use crate::ui::{Open, Ui};

/// What a `Pager` action asks for: a transcript block (`{item}`) or a titled
/// text (`{title, text}`), and optionally the pane `esc` gives the keyboard
/// back to (`return_to: {plugin, id}`, the same reference `Pane` focuses).
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Page {
    #[serde(flatten)]
    pub source: Source,
    #[serde(default)]
    pub return_to: Option<PaneRef>,
}

/// A pane by its owner and slot id.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PaneRef {
    pub plugin: String,
    pub id: String,
}

/// How [`Source`] reads off the wire; the sheet keeps the one enum.
#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum Wire {
    Item { item: ItemId },
    Text { title: String, text: String },
}

impl From<Wire> for Source {
    fn from(wire: Wire) -> Self {
        match wire {
            Wire::Item { item } => Source::Item(item),
            Wire::Text { title, text } => Source::Text { title, text },
        }
    }
}

impl Page {
    pub fn item(item: ItemId) -> Self {
        Self {
            source: Source::Item(item),
            return_to: None,
        }
    }
}

/// Open the sheet a page asks for. A block the viewed session does not hold
/// opens nothing. Without a pane named, `esc` goes back to what the sheet was
/// opened over: Browse as it was left, or the focused pane.
pub fn open(ui: &mut Ui, tree: &Tree, page: Page, now: Now) {
    if let Source::Item(id) = &page.source
        && tree.viewed().transcript.get(id).is_none()
    {
        return;
    }
    let return_to = match &page.return_to {
        Some(pane) => placed(tree, pane).map(Return::Pane),
        None => behind(ui),
    };
    let pager = Pager::new(page.source, return_to);
    ui.focus.show(Open::Pager(pager), now.instant);
}

fn placed(tree: &Tree, pane: &PaneRef) -> Option<String> {
    crate::pane::placed(tree.viewed())
        .into_iter()
        .find(|placed| placed.plugin == pane.plugin && placed.id == pane.id)
        .map(|placed| placed.key)
}

fn behind(ui: &Ui) -> Option<Return> {
    match (&ui.focus, ui.focus.open()) {
        (_, open @ Open::Browse(_)) => Some(Return::Layer(Box::new(open.clone()))),
        (Focus::Pane(key), _) => Some(Return::Pane(key.clone())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use serde_json::json;

    fn parsed(args: serde_json::Value) -> Page {
        serde_json::from_value(args).unwrap()
    }

    fn with_pane() -> bingo_sdk::SessionState {
        let mut state = state();
        state.transcript = vec![user("i1", "first")].into_iter().collect();
        let slot = serde_json::from_value(json!({
            "kind":"pane", "id":"memory", "title":"memory", "focus":true,
            "asked":true, "closeOnEscape":false, "holdToasts":false, "rows":8,
            "view":{"kind":"Text", "children":["rows"]}
        }))
        .unwrap();
        state.signals.entry("owner".into()).or_default().insert(
            "memory".into(),
            bingo_sdk::slot::with(json!({}), [slot]).into(),
        );
        state
    }

    #[test]
    fn the_item_form_reads_a_transcript_block() {
        assert_eq!(
            parsed(json!({"item":"i1"})),
            Page::item(ItemId::from_raw("i1"))
        );
    }

    #[test]
    fn the_text_form_reads_a_title_and_a_body_and_names_its_pane() {
        assert_eq!(
            parsed(json!({"title":"t","text":"body","return_to":{"plugin":"owner","id":"memory"}})),
            Page {
                source: Source::Text {
                    title: "t".into(),
                    text: "body".into()
                },
                return_to: Some(PaneRef {
                    plugin: "owner".into(),
                    id: "memory".into()
                }),
            }
        );
        assert!(serde_json::from_value::<Page>(json!({"title":"t"})).is_err());
    }

    #[test]
    fn an_item_opens_only_when_the_viewed_session_holds_it() {
        let tree = solo(&with_pane());
        let (mut ui, now) = scene();
        open(&mut ui, &tree, Page::item(ItemId::from_raw("gone")), now);
        assert!(!ui.focus.showing());
        open(&mut ui, &tree, parsed(json!({"item":"i1"})), now);
        assert!(
            matches!(ui.focus.open(), Open::Pager(p) if p.source == Source::Item(ItemId::from_raw("i1")) && p.return_to.is_none())
        );
    }

    #[test]
    fn a_named_pane_is_where_esc_goes_back_to() {
        let state = with_pane();
        let tree = solo(&state);
        let key = crate::pane::placed(&state)[0].key.clone();
        let (mut ui, now) = scene();
        let page = json!({"title":"t","text":"body","return_to":{"plugin":"owner","id":"memory"}});
        open(&mut ui, &tree, parsed(page), now);
        assert!(
            matches!(ui.focus.open(), Open::Pager(p) if p.return_to == Some(Return::Pane(key.clone())))
        );
        crate::pager::keys(
            &mut ui,
            &tree,
            key_press(crossterm::event::KeyCode::Esc),
            now,
        );
        assert!(matches!(&ui.focus, Focus::Pane(focused) if focused == &key));
    }

    #[test]
    fn an_unnamed_page_goes_back_to_the_focused_pane() {
        let state = with_pane();
        let tree = solo(&state);
        let key = crate::pane::placed(&state)[0].key.clone();
        let (mut ui, now) = scene();
        ui.focus = Focus::Pane(key.clone());
        open(
            &mut ui,
            &tree,
            parsed(json!({"title":"t","text":"body"})),
            now,
        );
        assert!(
            matches!(ui.focus.open(), Open::Pager(p) if p.return_to == Some(Return::Pane(key)))
        );
    }

    fn key_press(code: crossterm::event::KeyCode) -> crossterm::event::KeyEvent {
        crossterm::event::KeyEvent::new(code, crossterm::event::KeyModifiers::NONE)
    }
}
