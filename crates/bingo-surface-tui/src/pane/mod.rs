//! Pane declarations and the surface state belonging to their controls.
mod draw;
mod mouse;
mod moved;
pub mod seat;
use crate::{composer::Composer, effect::Effect, focus::Focus, theme, ui::Ui};
use bingo_sdk::{
    Action, SessionState, View,
    slot::{Shown, Slot},
    view::{InputProps, SelectProps},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
pub use draw::{draw, status};
pub use mouse::mouse;
use ratatui::layout::Rect;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default)]
pub struct State {
    order: Vec<String>,
    moved: BTreeMap<String, bingo_sdk::slot::Place>,
    scroll: BTreeMap<String, usize>,
    selection: Option<(String, crate::select::Run)>,
    selected: BTreeMap<String, usize>,
    editors: BTreeMap<(String, usize), Composer>,
}
#[derive(Clone)]
pub struct Pane {
    pub key: String,
    pub plugin: String,
    pub id: String,
    pub title: String,
    pub focus: bool,
    pub close: bool,
    pub hold: bool,
    pub asked: bool,
    pub place: Vec<bingo_sdk::slot::Place>,
    pub size: bingo_sdk::slot::PaneSize,
    pub view: Shown,
}
pub fn heading_rows(pane: &Pane) -> u16 {
    u16::from(!pane.title.is_empty())
}
pub fn placed(state: &SessionState) -> Vec<Pane> {
    bingo_sdk::slot::placed(state)
        .into_iter()
        .filter_map(|placed| {
            if let Slot::Pane {
                id,
                title,
                focus,
                close_on_escape,
                hold_toasts,
                asked,
                place,
                size,
                view,
            } = placed.slot
            {
                Some(Pane {
                    key: format!("{}/{}/{}", state.summary.id, placed.plugin, id),
                    plugin: placed.plugin,
                    id,
                    title,
                    focus,
                    close: close_on_escape,
                    hold: hold_toasts,
                    asked,
                    place,
                    size,
                    view,
                })
            } else {
                None
            }
        })
        .collect()
}
impl State {
    pub fn visible(&self, pane: &Pane) -> bool {
        pane.asked
    }
    pub fn sync(&mut self, state: &SessionState, focus: &mut Focus) {
        let panes = placed(state);
        let keys: BTreeSet<_> = panes.iter().map(|pane| pane.key.clone()).collect();
        self.order.retain(|key| keys.contains(key));
        self.scroll.retain(|key, _| keys.contains(key));
        if self
            .selection
            .as_ref()
            .is_some_and(|(key, _)| !keys.contains(key))
        {
            self.selection = None;
        }
        self.selected.retain(|key, _| keys.contains(key));
        self.editors.retain(|(key, _), _| keys.contains(key));
        if matches!(focus,Focus::Pane(key) if !keys.contains(key)) {
            *focus = Focus::Composer;
        }
        for pane in panes {
            if !pane.asked {
                continue;
            }
            if self.order.contains(&pane.key) {
                continue;
            }
            // A closing palette still owns focus; keep the opening request pending.
            if pane.focus && !matches!(focus, Focus::Composer) {
                continue;
            }
            self.order.push(pane.key.clone());
            if pane.asked && pane.focus && matches!(focus, Focus::Composer) {
                *focus = Focus::Pane(pane.key);
            }
        }
    }
    pub fn holding(&self, state: &SessionState) -> bool {
        placed(state)
            .iter()
            .any(|pane| pane.hold && self.visible(pane))
    }
}
pub fn wanted(ui: &Ui, state: &SessionState) -> Vec<Pane> {
    let mut panes: Vec<_> = placed(state)
        .into_iter()
        .filter(|pane| ui.placed.panes.visible(pane))
        .collect();
    panes.sort_by_key(|pane| {
        ui.placed
            .panes
            .order
            .iter()
            .position(|key| key == &pane.key)
    });
    panes
}
pub fn seating(ui: &Ui, state: &SessionState, area: Rect) -> seat::Seating {
    seat::seat(area, &wanted(ui, state), &ui.placed.panes.moved)
}
pub fn waiting(ui: &Ui, state: &SessionState) -> Vec<Pane> {
    placed(state)
        .into_iter()
        .filter(|pane| !ui.placed.panes.visible(pane))
        .collect()
}
fn view<'a>(
    ui: &'a Ui,
    state: &SessionState,
    pane: &'a Pane,
    seated: &seat::Seated,
) -> Option<&'a View> {
    match &pane.view {
        Shown::View(view) => Some(view),
        Shown::Pull { .. } => {
            let shown = shown(ui, pane, seated);
            if !crate::component::pane::matched(state, &shown) {
                return None;
            }
            let Shown::Pull { request } = shown else {
                return None;
            };
            ui.placed
                .pulled
                .display(&crate::pulled::Key::seated(state, &request, seated))
        }
    }
}
fn shown(ui: &Ui, pane: &Pane, seated: &seat::Seated) -> Shown {
    crate::component::pane::shown(
        pane,
        matches!(&ui.focus, Focus::Pane(key) if key == &pane.key),
        seated,
        ui.placed
            .panes
            .scroll
            .get(&pane.key)
            .copied()
            .unwrap_or_default(),
    )
}
#[derive(Clone)]
enum Control {
    Button(Option<char>, Action),
    Input(InputProps),
    Select(SelectProps),
}
fn controls(view: &View, out: &mut Vec<Control>) {
    match view {
        View::Box { children, layout } if layout.display.as_deref() != Some("none") => {
            for child in children {
                controls(child, out);
            }
        }
        View::Panel { child, .. } => controls(child, out),
        View::Button { button, .. } => {
            if let Some(action) = &button.on_press {
                out.push(Control::Button(button.hotkey, action.clone()));
            }
        }
        View::Input { input } => out.push(Control::Input(input.as_ref().clone())),
        View::Select { select } => out.push(Control::Select(select.as_ref().clone())),
        _ => {}
    }
}
fn edited(
    view: &mut View,
    target: usize,
    editor: Option<&Composer>,
    focused: bool,
    index: &mut usize,
) {
    match view {
        View::Box { children, layout } if layout.display.as_deref() != Some("none") => {
            for child in children {
                edited(child, target, editor, focused, index);
            }
        }
        View::Panel { child, .. } => edited(child, target, editor, focused, index),
        View::Input { input } => {
            if *index == target {
                if let Some(editor) = editor {
                    input.value = Some(editor.text().into());
                }
                if focused {
                    mark_label(&mut input.label);
                    input.value = Some(caret(editor, input.value.as_deref().unwrap_or_default()));
                }
            }
            *index += 1;
        }
        View::Select { select } => {
            if *index == target && focused {
                mark_label(&mut select.label);
            }
            *index += 1;
        }
        View::Button { button, children } if button.on_press.is_some() => {
            if *index == target && focused {
                if button.label.is_none() {
                    button.label = Some(bingo_sdk::view::text_children(children));
                }
                mark_label(&mut button.label);
            }
            *index += 1;
        }
        _ => {}
    }
}
fn mark_label(label: &mut Option<String>) {
    *label = Some(format!(
        "{} {}",
        theme::cursor(),
        label.as_deref().unwrap_or_default()
    ));
}
fn caret(editor: Option<&Composer>, value: &str) -> String {
    use unicode_width::UnicodeWidthChar;
    let mut resting = Composer::default();
    resting.set(value);
    let layout = editor.unwrap_or(&resting).layout(usize::MAX / 2);
    let mut lines = layout.lines;
    if let Some(line) = lines.get_mut(layout.cursor.0) {
        let mut width = 0;
        let byte = line
            .char_indices()
            .find_map(|(byte, c)| {
                if width >= layout.cursor.1 {
                    Some(byte)
                } else {
                    width += c.width().unwrap_or_default();
                    None
                }
            })
            .unwrap_or(line.len());
        line.insert_str(byte, theme::caret());
    }
    lines.join("\n")
}
pub fn key_handler(ui: &Ui, state: &SessionState, key: KeyEvent) -> Option<Action> {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    let Focus::Pane(focused) = &ui.focus else {
        return None;
    };
    let pane = placed(state)
        .into_iter()
        .find(|pane| &pane.key == focused)?;
    if key.code == KeyCode::Esc && pane.close {
        return None;
    }
    let seated = ui
        .shown
        .seating
        .panes
        .iter()
        .find(|seat| &seat.key == focused)?;
    let View::Box { layout, .. } = view(ui, state, &pane, seated)? else {
        return None;
    };
    let mut action = layout.on_key.clone()?;
    // A key names only itself: a key queued behind a redraw would carry a
    // drawing the site has already replaced. The site keeps its own cursor.
    action.args.as_object_mut()?.insert(
        "value".into(),
        serde_json::json!({"key": crate::keys::commands::chord(key)?}),
    );
    Some(action)
}

pub fn key(ui: &mut Ui, state: &SessionState, key: KeyEvent) -> Vec<Effect> {
    let Focus::Pane(focused) = &ui.focus else {
        return Vec::new();
    };
    let focused = focused.clone();
    let Some(pane) = placed(state).into_iter().find(|pane| pane.key == focused) else {
        ui.focus = Focus::Composer;
        return Vec::new();
    };
    if key.code == KeyCode::Esc
        && state.busy()
        && matches!(&pane.view, Shown::Pull { request } if request.component == bingo_sdk::render::Component::Status)
    {
        return vec![Effect::Interrupt];
    }
    if key.code == KeyCode::Esc {
        ui.placed.panes.selection = None;
        if pane.close {
            return submit(
                Action {
                    name: format!("{}:ui", pane.plugin),
                    args: serde_json::json!({"handler":pane.id,"event":"close"}),
                },
                None,
            );
        }
        ui.focus = Focus::Composer;
        return Vec::new();
    }
    if key.code == KeyCode::Char('p') && key.modifiers == KeyModifiers::ALT {
        moved::cycle(ui, state, &focused);
        return Vec::new();
    }
    let Some(seated) = ui
        .shown
        .seating
        .panes
        .iter()
        .find(|seat| seat.key == focused)
    else {
        return Vec::new();
    };
    let Some(view) = view(ui, state, &pane, seated) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    controls(view, &mut items);
    if items.is_empty() {
        ui.focus = Focus::Composer;
        return Vec::new();
    }
    let selected = ui.placed.panes.selected.entry(focused.clone()).or_default();
    *selected = (*selected).min(items.len() - 1);
    if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
        *selected = if key.code == KeyCode::BackTab {
            (*selected + items.len() - 1) % items.len()
        } else {
            (*selected + 1) % items.len()
        };
        return Vec::new();
    }
    let index = *selected;
    if !matches!(items[index], Control::Input(_))
        && let KeyCode::Char(letter) = key.code
        && key.modifiers.is_empty()
        && let Some(Control::Button(_, action)) = items
            .iter()
            .find(|item| matches!(item,Control::Button(Some(hotkey),_) if *hotkey==letter))
    {
        return submit(action.clone(), None);
    }
    match &items[index] {
        Control::Button(_, action) if key.code == KeyCode::Enter => submit(action.clone(), None),
        Control::Input(input) => {
            let editor = ui
                .placed
                .panes
                .editors
                .entry((focused, index))
                .or_insert_with(|| {
                    let mut editor = Composer::default();
                    editor.set(input.value.as_deref().unwrap_or_default());
                    editor
                });
            input_key(editor, input, key)
        }
        Control::Select(select) => select_key(select, key),
        _ => Vec::new(),
    }
}
fn input_key(editor: &mut Composer, input: &InputProps, key: KeyEvent) -> Vec<Effect> {
    if key.code == KeyCode::Enter {
        return input
            .on_submit
            .clone()
            .map(|action| submit(action, Some(editor.text().into())))
            .unwrap_or_default();
    }
    let before = editor.text().to_owned();
    match (key.code, key.modifiers) {
        (KeyCode::Char('a'), KeyModifiers::CONTROL) | (KeyCode::Home, _) => editor.home(),
        (KeyCode::Char('e'), KeyModifiers::CONTROL) | (KeyCode::End, _) => editor.end(),
        (KeyCode::Char('w'), KeyModifiers::CONTROL) => editor.delete_word_left(),
        (KeyCode::Char('u'), KeyModifiers::CONTROL) => editor.delete_to_line_start(),
        (KeyCode::Char('k'), KeyModifiers::CONTROL) => editor.delete_to_line_end(),
        (KeyCode::Char(c), modifiers)
            if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            editor.insert(&c.to_string())
        }
        (KeyCode::Backspace, _) => editor.backspace(),
        (KeyCode::Delete, _) => editor.delete(),
        (KeyCode::Left, _) => editor.left(),
        (KeyCode::Right, _) => editor.right(),
        _ => {}
    }
    if before == editor.text() {
        return Vec::new();
    }
    input
        .on_input
        .clone()
        .map(|action| submit(action, Some(editor.text().into())))
        .unwrap_or_default()
}
fn select_key(select: &SelectProps, key: KeyEvent) -> Vec<Effect> {
    let options = select.options.as_deref().unwrap_or_default();
    if options.is_empty() {
        return Vec::new();
    }
    let index = options
        .iter()
        .position(|option| Some(&option.value) == select.value.as_ref())
        .unwrap_or_default();
    let index = match key.code {
        KeyCode::Up => (index + options.len() - 1) % options.len(),
        KeyCode::Down => (index + 1) % options.len(),
        KeyCode::Enter => index,
        _ => return Vec::new(),
    };
    select
        .on_select
        .clone()
        .map(|action| submit(action, Some(options[index].value.clone())))
        .unwrap_or_default()
}
fn submit(mut action: Action, value: Option<String>) -> Vec<Effect> {
    if let Some(value) = value {
        action.args["value"] = value.into();
    }
    vec![Effect::Submit(crate::acted(action))]
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod integration_tests;

#[cfg(test)]
mod focus_tests;

#[cfg(test)]
#[path = "tests/status.rs"]
mod status_tests;
