use bingo_sdk::{ActionItem, component::Props};
use ratatui::{layout::Rect, text::Line};
#[derive(Default)]
pub struct Drawing {
    pub lines: Vec<Line<'static>>,
    mask: Vec<Line<'static>>,
    actions: Vec<ActionItem>,
}
pub fn draw(
    state: &bingo_sdk::SessionState,
    ui: &crate::ui::Ui,
    frame: &mut crate::layout::Layout,
) -> Drawing {
    use bingo_sdk::component::{AbovePromptProps, SiteScroll, SiteView};
    if ui.activity_hidden {
        return Drawing::default();
    }
    let area = frame.area();
    let max_rows = area.height / 2;
    let props = Props::AbovePrompt(AbovePromptProps {
        has_survey: false,
        is_working: state.busy(),
        max_rows,
        body_columns: area.width,
        scroll: SiteScroll {
            offset: 0,
            body_rows: usize::from(max_rows),
        },
        view: SiteView { agent_id: None },
    });
    let Some(key) = super::key(state, "AbovePrompt", &props, area.width, max_rows) else {
        return Drawing::default();
    };
    let mut lines = super::draw(
        &props,
        ui.placed.pulled.get(&key),
        usize::from(area.width),
        |_, _| Vec::new(),
    );
    frame.pulls.push(key.clone());
    lines.truncate(usize::from(max_rows));
    let (mask, actions) = ui
        .placed
        .pulled
        .get(&key)
        .map(|view| crate::views::targets::render(view, usize::from(area.width)))
        .unwrap_or_default();
    Drawing {
        lines,
        mask,
        actions,
    }
}

pub fn place(drawing: &Drawing, frame: &mut crate::layout::Layout, area: Rect, offset: usize) {
    let offset = offset.min(usize::from(area.height)) as u16;
    let area = Rect::new(area.x, area.y + offset, area.width, area.height - offset);
    frame.band_actions = crate::views::targets::cells(&drawing.mask, &drawing.actions, area);
    if let Some(foot) = frame.foot {
        frame
            .band_actions
            .retain(|mark| !mark.area.intersects(foot));
    }
}
pub fn press(
    ui: &crate::ui::Ui,
    at: ratatui::layout::Position,
) -> Option<Vec<crate::effect::Effect>> {
    ui.shown
        .band_actions
        .iter()
        .find(|mark| mark.area.contains(at))
        .map(fire)
}
fn fire(mark: &crate::views::targets::Target) -> Vec<crate::effect::Effect> {
    vec![crate::effect::Effect::Submit(bingo_sdk::Input::Action {
        action: mark.item.action.clone(),
    })]
}
pub fn hotkey(
    ui: &crate::ui::Ui,
    key: crossterm::event::KeyEvent,
) -> Option<Vec<crate::effect::Effect>> {
    use crossterm::event::{KeyCode, KeyModifiers};
    if !matches!(ui.focus, crate::focus::Focus::Composer) || key.modifiers != KeyModifiers::ALT {
        return None;
    }
    let KeyCode::Char(ch) = key.code else {
        return None;
    };
    ui.shown
        .band_actions
        .iter()
        .find(|mark| mark.item.key == Some(ch))
        .map(fire)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    #[test]
    fn only_unclaimed_alt_hotkeys_fire_while_composing() {
        let state = state();
        let tree = solo(&state);
        let (mut ui, now) = scene();
        let item = ActionItem {
            label: "Hide".into(),
            key: Some('h'),
            action: bingo_sdk::Action {
                name: "band:ui".into(),
                args: serde_json::Value::Null,
            },
        };
        ui.shown.band_actions.push(crate::views::targets::Target {
            area: Rect::new(0, 0, 8, 1),
            item,
        });
        let typed = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
        assert!(crate::input::on_key(&mut ui, &tree, typed, now).is_empty());
        assert_eq!(ui.composer.text(), "h");
        let alt = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::ALT);
        assert!(matches!(
            crate::input::on_key(&mut ui, &tree, alt, now).as_slice(),
            [crate::effect::Effect::Submit(
                bingo_sdk::Input::Action { .. }
            )]
        ));
        ui.shown.band_actions[0].item.key = Some('b');
        let own = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::ALT);
        assert!(crate::input::on_key(&mut ui, &tree, own, now).is_empty());
        ui.shown.band_actions[0].item.key = Some('g');
        let global = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL);
        assert!(
            !crate::input::on_key(&mut ui, &tree, global, now)
                .iter()
                .any(|effect| matches!(
                    effect,
                    crate::effect::Effect::Submit(bingo_sdk::Input::Action { .. })
                ))
        );
    }
    #[test]
    fn action_cells_follow_padding_clipping_and_hidden_boxes() {
        let view: bingo_sdk::View = serde_json::from_value(serde_json::json!({"kind":"Box","padding":2,"children":[{"kind":"Button","children":["same"],"hotkey":"h","onPress":{"name":"band:ui","args":null}},{"kind":"Box","display":"none","children":[{"kind":"Button","children":["same"],"onPress":{"name":"hidden:ui","args":null}}]}]})).unwrap();
        let (mask, actions) = crate::views::targets::render(&view, 30);
        let marks = crate::views::targets::cells(&mask, &actions, Rect::new(5, 10, 8, 3));
        assert!(!marks.is_empty());
        assert!(marks.iter().all(|m| m.area.y == 12
            && m.area.x >= 7
            && m.area.right() <= 13
            && m.item.action.name == "band:ui"));
        assert!(crate::views::targets::cells(&mask, &actions, Rect::new(5, 10, 30, 2)).is_empty());
    }
}
