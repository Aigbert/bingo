//! A second pass through the same layout maps visible cells to actions.
use super::{Marks, marked};
use bingo_sdk::{ActionItem, View};
use ratatui::{layout::Rect, text::Line};
use unicode_width::UnicodeWidthStr;

#[derive(Debug)]
pub struct Target {
    pub area: Rect,
    pub item: ActionItem,
}
const FIRST: u32 = 0xF0000;

pub fn render(view: &View, width: usize) -> (Vec<Line<'static>>, Vec<ActionItem>) {
    let targets = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let lines = marked(
        view,
        width,
        &Marks {
            targets: Some(targets.clone()),
            ..Marks::default()
        },
    );
    let actions = targets.borrow().clone();
    (lines.lines, actions)
}

pub(super) fn mask(view: &View, lines: Vec<Line<'static>>, marks: &Marks) -> Vec<Line<'static>> {
    let Some(targets) = &marks.targets else {
        return lines;
    };
    if matches!(view, View::Box { .. } | View::Panel { .. }) {
        return lines;
    }
    let symbol = if let View::Button { button, .. } = view
        && let Some(action) = &button.on_press
    {
        let mut targets = targets.borrow_mut();
        let symbol = char::from_u32(FIRST + targets.len() as u32).unwrap_or(' ');
        targets.push(ActionItem {
            label: String::new(),
            key: button.hotkey,
            action: action.clone(),
        });
        symbol
    } else {
        ' '
    };
    lines
        .into_iter()
        .map(|line| Line::from(symbol.to_string().repeat(line.width())))
        .collect()
}

pub fn cells(lines: &[Line<'_>], actions: &[ActionItem], area: Rect) -> Vec<Target> {
    let mut targets = Vec::new();
    for (row, line) in lines.iter().take(usize::from(area.height)).enumerate() {
        let mut column = 0usize;
        for span in &line.spans {
            for ch in span.content.chars() {
                if column >= usize::from(area.width) {
                    break;
                }
                if let Some(item) = (ch as u32)
                    .checked_sub(FIRST)
                    .and_then(|i| actions.get(i as usize))
                {
                    targets.push(Target {
                        area: Rect::new(area.x + column as u16, area.y + row as u16, 1, 1),
                        item: item.clone(),
                    });
                }
                column += ch.to_string().width();
            }
        }
    }
    targets
}
