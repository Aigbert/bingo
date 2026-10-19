//! A sheet's anatomy, shared by every owner. Rows and actions stay with the owner.
//! The viewport is the dock body's capacity, excluding its edge and bottom pad.
use crate::{
    View,
    pick::{self, Row},
    view::BoxStyle,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Press {
    Key(String),
    Pick(String),
}

impl Press {
    pub fn key(&self) -> Option<&str> {
        match self {
            Self::Key(key) => Some(key),
            Self::Pick(_) => None,
        }
    }
    pub fn picked(&self) -> Option<&str> {
        match self {
            Self::Pick(id) => Some(id),
            Self::Key(_) => None,
        }
    }
}

pub fn pressed(args: &Value) -> Option<Press> {
    let press = serde_json::from_value(args.get("value")?.clone()).ok()?;
    Some(match press {
        Press::Key(key) => Press::Key(crate::keys::normalize(&key).into()),
        press => press,
    })
}

pub const PERMISSION_MODE: &str = "PermissionMode";

pub const CONTENT_GUTTERS: u16 = 5;

pub struct Sheet {
    pub head: String,
    pub fact: Option<View>,
    pub top: Vec<View>,
    pub rows: Vec<Row>,
    pub selected: Option<String>,
    pub foot: Vec<View>,
    pub pick: bool,
}
impl Sheet {
    pub fn window(&self, rows: u16) -> std::ops::Range<usize> {
        self.window_with_limit(rows, 8)
    }
    fn window_with_limit(&self, rows: u16, max: usize) -> std::ops::Range<usize> {
        let max = capacity(rows, self.top.len(), max);
        let at = pick::selected(&self.rows, self.selected.as_deref());
        if self.pick {
            pick::list_window(self.rows.len(), at, max)
        } else {
            let start = at.min(self.rows.len().saturating_sub(max));
            start..self.rows.len().min(start + max)
        }
    }
    /// Ordinary lists show eight rows at most; the unpicked keys page may use forty.
    pub fn view(&self, columns: u16, rows: u16, glyphs: crate::glyph::GlyphSet) -> View {
        self.with_limit(columns, rows, 8, glyphs)
    }
    pub fn with_limit(
        &self,
        columns: u16,
        rows: u16,
        max: usize,
        glyphs: crate::glyph::GlyphSet,
    ) -> View {
        let at = pick::selected(&self.rows, self.selected.as_deref());
        let window = self.window_with_limit(rows, max);
        let head = Row {
            id: String::new(),
            left: pick::bold(&self.head, "fg"),
            right: self.fact.clone(),
        };
        let mut children = vec![pick::pick_line(&head, false, columns, glyphs), blank()];
        children.extend(self.top.iter().cloned().map(|v| inset(v, columns)));
        children.extend(
            window
                .clone()
                .map(|i| pick::pick_line(&self.rows[i], self.pick && i == at, columns, glyphs)),
        );
        if let Some(count) = pick::count(window, self.rows.len()) {
            children.push(inset(count, columns));
        }
        if !self.foot.is_empty() {
            children.push(blank());
            children.extend(self.foot.iter().cloned().map(|v| inset(v, columns)));
        }
        View::Box {
            children,
            layout: Box::new(BoxStyle {
                ground: Some("you".into()),
                ..Default::default()
            }),
        }
    }
}
/// Visible list capacity, also used when an owner fetches facts for visible rows.
pub fn capacity(rows: u16, top: usize, max: usize) -> usize {
    max.min(usize::from(rows).saturating_sub(4 + top).max(3))
}

fn blank() -> View {
    pick::text("", "fg")
}
fn inset(view: View, columns: u16) -> View {
    View::Box {
        children: vec![
            pick::cell(blank(), 3),
            View::Box {
                children: vec![view],
                layout: Box::new(BoxStyle {
                    width: Some(columns.saturating_sub(CONTENT_GUTTERS)),
                    ..Default::default()
                }),
            },
        ],
        layout: Box::new(BoxStyle {
            flex_direction: Some(crate::view::Direction::Row),
            ..Default::default()
        }),
    }
}

/// Give each visible row its owner's key handler, carrying its stable identity.
/// A row click is the same choice as moving to that row and pressing Enter.
pub fn clicks(mut view: View) -> View {
    let View::Box { children, layout } = &mut view else {
        return view;
    };
    let Some(action) = &layout.on_key else {
        return view;
    };
    for child in children {
        let View::Box { children, .. } = child else {
            continue;
        };
        for body in children {
            let View::Box { layout, .. } = body else {
                continue;
            };
            let Some(id) = layout.key.as_deref().filter(|id| !id.is_empty()) else {
                continue;
            };
            let Some(action) = action
                .clone()
                .with_value(serde_json::json!(Press::Pick(id.into())))
            else {
                continue;
            };
            click_cells(body, &action);
        }
    }
    view
}

fn click_cells(body: &mut View, action: &crate::Action) {
    let View::Box { children, .. } = body else {
        return;
    };
    for cell in children {
        let View::Box { children, layout } = cell else {
            continue;
        };
        for text in children {
            if !matches!(text, View::Text { .. }) {
                continue;
            }
            *text = View::Button {
                children: vec![crate::view::TextChild::Node(Box::new(pick::pad(
                    text.clone(),
                    usize::from(layout.width.unwrap_or(0)),
                )))],
                button: Box::new(crate::view::ButtonProps {
                    variant: Some("plain".into()),
                    on_press: Some(action.clone()),
                    ..Default::default()
                }),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sheet() -> Sheet {
        Sheet {
            head: "Models".into(),
            fact: None,
            top: vec![],
            rows: (0..20)
                .map(|i| Row::new(i.to_string(), pick::text(format!("model {i}"), "fg")))
                .collect(),
            selected: Some("19".into()),
            foot: vec![],
            pick: true,
        }
    }
    #[test]
    fn clicks_keep_valid_text_nodes_and_the_visible_row_identity() {
        let mut view = sheet().view(80, 12, Default::default());
        if let View::Box { layout, .. } = &mut view {
            layout.on_key = Some(crate::Action {
                name: "example:ui".into(),
                args: serde_json::json!({"handler":"sheet"}),
            });
        }
        let view = clicks(view);
        crate::view::answer_ok(
            crate::render::Component::Pane,
            &serde_json::json!({}),
            "example",
            None,
            &view,
        )
        .unwrap();
        let wire = serde_json::to_value(&view).unwrap();
        let wire = wire.to_string();
        assert!(wire.contains(r#""pick":"19""#));
        assert!(!wire.contains(r#""pick":"11""#));
        assert!(!wire.contains(r#""pick":"head""#));
        assert!(view.fold().contains("model 19"));
    }
    #[test]
    fn one_builder_caps_the_list_and_keeps_the_picked_identity() {
        let view = sheet().view(100, 18, Default::default());
        assert!(view.fold().contains("8 of 20"));
        assert!(view.fold().contains("model 19"));
        assert!(!view.fold().contains("model 11"));
        assert!(
            serde_json::to_string(&view)
                .unwrap()
                .contains("\"ground\":\"pick\"")
        );
    }
    #[test]
    fn the_sheet_window_clamps_at_both_ends() {
        let mut sheet = sheet();
        assert_eq!(sheet.window(18), 12..20);
        sheet.selected = Some("0".into());
        assert_eq!(sheet.window(18), 0..8);
        assert!(
            sheet
                .view(100, 18, Default::default())
                .fold()
                .contains("model 0")
        );
        assert!(
            !sheet
                .view(100, 18, Default::default())
                .fold()
                .contains("model 8")
        );
    }
    #[test]
    fn three_rows_are_the_minimum_even_in_a_tiny_viewport() {
        assert!(
            sheet()
                .view(10, 0, Default::default())
                .fold()
                .contains("3 of 20")
        );
    }
    #[test]
    fn unpicked_pages_scroll_without_a_pick_ground() {
        let mut sheet = sheet();
        sheet.pick = false;
        let view = sheet.with_limit(100, 18, 40, crate::glyph::GlyphSet::Ascii);
        assert!(view.fold().contains("14 of 20"));
        assert!(
            !serde_json::to_string(&view)
                .unwrap()
                .contains("\"ground\":\"pick\"")
        );
    }
}
