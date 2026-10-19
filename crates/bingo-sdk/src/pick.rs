//! The one window for dock lists: sheets, command and mention menus (M196),
//! and an ask's options (M197). Owners supply rows and keep selection by id.
use crate::{
    View,
    view::{BoxStyle, Direction, TextChild, TextStyle},
};
use std::ops::Range;
use unicode_width::UnicodeWidthStr;

/// Centre the cursor, clamping at both ends. Empty lists have an empty window.
pub fn list_window(len: usize, selected: usize, max: usize) -> Range<usize> {
    let start = selected
        .min(len.saturating_sub(1))
        .saturating_sub(max / 2)
        .min(len.saturating_sub(max));
    start..len.min(start.saturating_add(max))
}

/// A row's stable identity and its two aligned runs. The owner styles its words.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub id: String,
    pub left: View,
    pub right: Option<View>,
}
impl Row {
    pub fn new(id: impl Into<String>, left: View) -> Self {
        Self {
            id: id.into(),
            left,
            right: None,
        }
    }
    pub fn right(mut self, right: View) -> Self {
        self.right = Some(right);
        self
    }
}

/// Resolve an identity in the current rows; a removed identity falls back to the first.
pub fn selected(rows: &[Row], id: Option<&str>) -> usize {
    id.and_then(|id| rows.iter().position(|row| row.id == id))
        .unwrap_or(0)
}

/// Move by identity, so inserting or sorting rows never changes what is picked.
pub fn moved(rows: &[Row], id: Option<&str>, down: bool) -> Option<String> {
    let at = selected(rows, id);
    let at = if down {
        at.saturating_add(1).min(rows.len().saturating_sub(1))
    } else {
        at.saturating_sub(1)
    };
    rows.get(at).map(|row| row.id.clone())
}

/// Draw the picked ground from TX−2 through W−RM, with the mark at TX−2.
pub fn pick_line(row: &Row, picked: bool, columns: u16, glyphs: crate::glyph::GlyphSet) -> View {
    let mark = crate::glyph::Glyph::Quote.spell(glyphs);
    let width = columns.saturating_sub(2);
    let right = row.right.clone().unwrap_or_else(|| text("", "dim"));
    let right_width = right
        .fold()
        .width()
        .min(usize::from(width.saturating_sub(3))) as u16;
    let mut children = vec![
        cell(text(if picked { mark } else { " " }, "glow"), 2),
        cell(row.left.clone(), width.saturating_sub(right_width + 3)),
        cell(right, right_width),
        cell(text(" ", "fg"), 1),
    ];
    let body = View::Box {
        children: std::mem::take(&mut children),
        layout: Box::new(BoxStyle {
            key: Some(row.id.clone()),
            width: Some(width),
            height: Some(1),
            flex_direction: Some(Direction::Row),
            ground: picked.then(|| "pick".into()),
            ..Default::default()
        }),
    };
    View::Box {
        children: vec![cell(text(" ", "fg"), 1), body],
        layout: Box::new(BoxStyle {
            flex_direction: Some(Direction::Row),
            height: Some(1),
            ..Default::default()
        }),
    }
}

/// The visible count, only when some rows are outside the window.
pub fn count(window: Range<usize>, total: usize) -> Option<View> {
    (window.len() < total).then(|| text(format!("{} of {total}", window.len()), "dim"))
}

pub fn text(value: impl Into<String>, ink: &str) -> View {
    View::Text {
        children: vec![TextChild::String(value.into())],
        style: TextStyle {
            color: Some(ink.into()),
            wrap: Some("truncate".into()),
            ..Default::default()
        },
    }
}
pub fn bold(value: impl Into<String>, ink: &str) -> View {
    let mut view = text(value, ink);
    if let View::Text { style, .. } = &mut view {
        style.bold = Some(true);
    }
    view
}
/// Width in terminal cells of a styled run.
pub fn width(view: &View) -> usize {
    view.fold().width()
}

/// Pad one styled run to a terminal-cell column without flattening its ink.
pub fn pad(mut view: View, width: usize) -> View {
    let spaces = width.saturating_sub(view.fold().width());
    if let View::Text { children, .. } = &mut view {
        children.push(TextChild::String(" ".repeat(spaces)));
        view
    } else {
        line([view, text(" ".repeat(spaces), "dim")])
    }
}

pub fn line(children: impl IntoIterator<Item = View>) -> View {
    View::Text {
        children: children
            .into_iter()
            .map(|v| TextChild::Node(Box::new(v)))
            .collect(),
        style: TextStyle {
            wrap: Some("truncate".into()),
            ..Default::default()
        },
    }
}
pub fn cell(view: View, width: u16) -> View {
    View::Box {
        children: vec![view],
        layout: Box::new(BoxStyle {
            width: Some(width),
            height: Some(1),
            ..Default::default()
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_window_centres_and_clamps_at_both_ends() {
        assert_eq!(list_window(20, 0, 8), 0..8);
        assert_eq!(list_window(20, 10, 8), 6..14);
        assert_eq!(list_window(20, 19, 8), 12..20);
        assert_eq!(list_window(0, 9, 8), 0..0);
        assert_eq!(list_window(2, 99, 8), 0..2);
    }
    #[test]
    fn counts_name_visible_rows_and_selection_survives_reordering() {
        assert_eq!(count(6..14, 20).unwrap().fold(), "8 of 20");
        assert!(count(0..2, 2).is_none());
        let rows = [
            Row::new("b", text("B", "fg")),
            Row::new("a", text("A", "fg")),
        ];
        assert_eq!(selected(&rows, Some("a")), 1);
        assert_eq!(moved(&rows, Some("a"), false).as_deref(), Some("b"));
    }
}
