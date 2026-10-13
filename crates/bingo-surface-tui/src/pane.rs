//! Pane placement and the editor belonging to the focused element.
use crate::{
    clock::Now, composer::Composer, effect::Effect, focus::Focus, layout::Layout, theme, ui::Ui,
    views,
};
use bingo_sdk::{
    Action, Input, SessionState, View,
    slot::{Shown, Slot},
    view::{InputProps, SelectProps},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default)]
pub struct State {
    opened: BTreeSet<String>,
    seen: BTreeSet<String>,
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
    pub rows: u16,
    pub view: Shown,
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
                rows,
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
                    rows,
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
        pane.asked || self.opened.contains(&pane.key)
    }
    pub fn sync(&mut self, state: &SessionState, focus: &mut Focus) {
        let panes = placed(state);
        let keys: BTreeSet<_> = panes.iter().map(|pane| pane.key.clone()).collect();
        self.opened.retain(|key| keys.contains(key));
        self.seen.retain(|key| keys.contains(key));
        self.selected.retain(|key, _| keys.contains(key));
        self.editors.retain(|(key, _), _| keys.contains(key));
        if matches!(focus,Focus::Pane(key) if !keys.contains(key)) {
            *focus = Focus::Composer;
        }
        for pane in panes {
            if self.seen.insert(pane.key.clone())
                && pane.asked
                && pane.focus
                && matches!(focus, Focus::Composer)
            {
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
pub fn height(ui: &Ui, state: &SessionState, screen: u16) -> u16 {
    placed(state)
        .iter()
        .filter(|pane| ui.panes.visible(pane))
        .map(|pane| pane.rows.max(2))
        .fold(0, u16::saturating_add)
        .min(screen / 3)
}
pub fn draw(ui: &Ui, state: &SessionState, frame: &mut Layout, area: Rect, now: Now) {
    let panes: Vec<_> = placed(state)
        .into_iter()
        .filter(|pane| ui.panes.visible(pane))
        .collect();
    let mut y = area.y;
    let count = panes.len().max(1) as u16;
    for pane in panes {
        let height = pane
            .rows
            .max(2)
            .min(area.height / count)
            .min(area.bottom().saturating_sub(y));
        let rect = Rect::new(area.x, y, area.width, height);
        y += height;
        if height == 0 {
            continue;
        }
        frame.panes.push((pane.key.clone(), rect));
        let selected = matches!(&ui.focus,Focus::Pane(key) if key==&pane.key);
        let title = if selected {
            format!("{} · Tab: next · Esc: close", pane.title)
        } else {
            pane.title.clone()
        };
        let mut lines = vec![Line::from(Span::styled(title, theme::bold()))];
        if let Shown::Pull { request } = &pane.view {
            frame.pulls.push(crate::pulled::Key::new(
                state,
                request,
                rect.width,
                rect.height.saturating_sub(1),
            ));
        }
        match ui.pulled.view(&pane.view, state, rect.width) {
            Some(view) => {
                let mut view = view.clone();
                let index = ui
                    .panes
                    .selected
                    .get(&pane.key)
                    .copied()
                    .unwrap_or_default();
                if let Some(editor) = ui.panes.editors.get(&(pane.key.clone(), index)) {
                    edited(&mut view, index, editor.text(), &mut 0);
                }
                lines.extend(views::marked(
                    &view,
                    usize::from(rect.width),
                    &views::Marks {
                        pending: ui.pending_ui.as_ref().map(|(_, action)| action.clone()),
                        ..views::Marks::at(now)
                    },
                ));
            }
            None => lines.push(Line::from(theme::ellipsis())),
        }
        frame.render_widget(Paragraph::new(lines), rect);
    }
}
pub fn waiting(ui: &Ui, state: &SessionState) -> Vec<Pane> {
    placed(state)
        .into_iter()
        .filter(|pane| !ui.panes.visible(pane))
        .collect()
}
pub fn click(ui: &mut Ui, at: ratatui::layout::Position) -> bool {
    let key = ui
        .shown
        .waiting
        .iter()
        .chain(&ui.shown.panes)
        .find(|(_, area)| area.contains(at))
        .map(|(key, _)| key.clone());
    if let Some(key) = key {
        ui.panes.opened.insert(key.clone());
        ui.focus = Focus::Pane(key);
        true
    } else {
        false
    }
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
        View::Input { input } => out.push(Control::Input(input.clone())),
        View::Select { select } => out.push(Control::Select(select.clone())),
        _ => {}
    }
}
fn edited(view: &mut View, target: usize, text: &str, index: &mut usize) {
    match view {
        View::Box { children, layout } if layout.display.as_deref() != Some("none") => {
            for child in children {
                edited(child, target, text, index);
            }
        }
        View::Panel { child, .. } => edited(child, target, text, index),
        View::Input { input } => {
            if *index == target {
                input.value = Some(text.into());
            }
            *index += 1;
        }
        View::Select { .. } => *index += 1,
        View::Button { button, .. } if button.on_press.is_some() => *index += 1,
        _ => {}
    }
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
    if key.code == KeyCode::Esc {
        ui.focus = Focus::Composer;
        if pane.close {
            return submit(
                Action {
                    name: format!("{}:ui", pane.plugin),
                    args: serde_json::json!({"handler":pane.id,"event":"close"}),
                },
                None,
            );
        }
        return Vec::new();
    }
    let width = ui
        .shown
        .panes
        .iter()
        .find(|(id, _)| id == &focused)
        .map_or(0, |(_, area)| area.width);
    let Some(view) = ui.pulled.view(&pane.view, state, width) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    controls(view, &mut items);
    if items.is_empty() {
        return Vec::new();
    }
    let selected = ui.panes.selected.entry(focused.clone()).or_default();
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
            let editor = ui.panes.editors.entry((focused, index)).or_insert_with(|| {
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
    vec![Effect::Submit(Input::Action { action })]
}

#[cfg(test)]
mod tests {
    use crate::test_support::*;
    use bingo_sdk::{
        View,
        slot::{Shown, Slot},
    };
    fn pane(asked: bool) -> Slot {
        Slot::Pane {
            id: "counter".into(),
            title: "Counter pane".into(),
            focus: true,
            close_on_escape: true,
            hold_toasts: false,
            asked,
            rows: 20,
            view: Shown::View(View::text("Count: 0")),
        }
    }
    fn with(slot: Slot) -> bingo_sdk::SessionState {
        let mut state = state();
        state.signals.entry("counter".into()).or_default().insert(
            "pane".into(),
            bingo_sdk::slot::with(serde_json::json!({}), [slot]),
        );
        state
    }
    #[test]
    fn asked_panes_are_inline_at_both_widths_and_unasked_panes_wait_for_a_click() {
        for width in [80, 200] {
            let (mut ui, now) = scene();
            let tree = solo(&with(pane(true)));
            let screen = draw_tree(width, 24, &tree, &mut ui, now);
            assert_eq!(ui.shown.panes[0].1.height, 8);
            assert_eq!(ui.shown.panes[0].1.bottom(), ui.shown.regions.composer.y);
            assert!(
                screen.contains("Counter pane") && screen.contains("Count: 0"),
                "{screen}"
            );
        }
        let (mut ui, now) = scene();
        let tree = solo(&with(pane(false)));
        let screen = draw_tree(200, 24, &tree, &mut ui, now);
        assert!(screen.contains("Counter pane (waiting)"), "{screen}");
        assert!(!screen.contains("Count: 0"));
        crate::pointer::on_mouse(
            &mut ui,
            &tree,
            crossterm::event::MouseEvent {
                kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
                column: 0,
                row: 23,
                modifiers: crossterm::event::KeyModifiers::NONE,
            },
            now,
        );
        assert!(draw_tree(200, 24, &tree, &mut ui, now).contains("Count: 0"));
    }
    #[test]
    fn pane_keys_use_handlers_and_an_independent_composer() {
        use bingo_sdk::{
            Action,
            view::{ButtonProps, InputProps},
        };
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let action = |event: &str| Action {
            name: "counter:ui".into(),
            args: serde_json::json!({"handler":"token","event":event}),
        };
        let mut slot = pane(true);
        if let Slot::Pane { view, .. } = &mut slot {
            *view = View::column(vec![
                View::Button {
                    children: vec![],
                    button: ButtonProps {
                        hotkey: Some('i'),
                        on_press: Some(action("press")),
                        ..Default::default()
                    },
                },
                View::Input {
                    input: InputProps {
                        value: Some("A".into()),
                        on_input: Some(action("input")),
                        ..Default::default()
                    },
                },
            ])
            .into();
        }
        let state = with(slot);
        let tree = solo(&state);
        let (mut ui, now) = scene();
        ui.sync_panes(&state);
        draw_tree(80, 24, &tree, &mut ui, now);
        let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
        let fired = crate::input::on_key(&mut ui, &tree, key(KeyCode::Char('i')), now);
        assert!(
            matches!(&fired[..],[crate::effect::Effect::Submit(bingo_sdk::Input::Action{action})] if action.args["event"]=="press")
        );
        crate::input::on_key(&mut ui, &tree, key(KeyCode::Tab), now);
        let typed = crate::input::on_key(&mut ui, &tree, key(KeyCode::Char('b')), now);
        assert!(
            matches!(&typed[..],[crate::effect::Effect::Submit(bingo_sdk::Input::Action{action})] if action.args["value"]=="Ab")
        );
        assert!(ui.composer.is_empty());
        assert!(draw_tree(80, 24, &tree, &mut ui, now).contains("Ab"));
        let closed = crate::input::on_key(&mut ui, &tree, key(KeyCode::Esc), now);
        assert!(
            matches!(&closed[..],[crate::effect::Effect::Submit(bingo_sdk::Input::Action{action})] if action.args["event"]=="close")
        );
    }
}
