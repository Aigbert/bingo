use crate::{
    Shared,
    rows::{Row, Target},
    state::{Cursor, Sheet},
};
use bingo_sdk::{component::SessionsProps, *};
use serde_json::{Value, json};
pub(crate) struct Drawing(pub Shared);
#[async_trait::async_trait]
impl Interceptor for Drawing {
    fn id(&self) -> &str {
        "sessions"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"component":"Sessions"}),
            ..LinkMatcher::at(catalogue::UI_RENDER)
        }
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        _: &HookContext,
        value: Value,
        _: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let request: render::RenderRequest = serde_json::from_value(value).map_err(crate::error)?;
        let props: SessionsProps = serde_json::from_value(request.props).map_err(crate::error)?;
        let mut state = self.0.lock().map_err(crate::error)?;
        let cursor = state
            .cursors
            .entry((props.session.clone(), request.request_id.clone()))
            .or_default();
        cursor.sync(&props.query);
        let rows = crate::rows::listed(&props, cursor);
        if cursor.selected.is_none() {
            cursor.selected = rows.first().map(|row| row.target.clone());
        }
        serde_json::to_value(tree(
            &props,
            &rows,
            cursor,
            &request.request_id,
            request.viewport.rows,
            request.viewport.columns,
        ))
        .map_err(crate::error)
    }
}
pub(crate) fn tree(
    props: &SessionsProps,
    rows: &[Row],
    cursor: &Cursor,
    request: &str,
    height: u16,
    width: u16,
) -> View {
    let scope = if props.scope == component::SessionsScope::Here {
        "here (tab: all)"
    } else {
        "all (tab: here)"
    };
    let total = props.sessions.len().min(props.limit);
    let count = rows
        .iter()
        .filter(|r| matches!(r.target, Target::Session(_)))
        .count();
    let mut children = vec![pair(
        dim(format!(" sessions · {scope} · {}", props.query)),
        dim(format!(
            "{count} of {total}{}",
            if props.sessions.len() > props.limit {
                "+"
            } else {
                ""
            }
        )),
        width,
    )];
    let room = usize::from(height).saturating_sub(3);
    let lines = match &cursor.sheet {
        Sheet::None | Sheet::Rename { .. } => directory(rows, cursor, room, width, props, request),
        Sheet::Loading(_) => vec![dim(" loading…")],
        Sheet::Preview(text) => text
            .lines()
            .take(room)
            .map(|s| View::text(format!(" {s}")))
            .collect(),
        Sheet::Delete(doomed) => confirmation(doomed, room),
    };
    children.extend(lines);
    children.push(dim("─".repeat(usize::from(width))));
    children.push(dim(hint(cursor)));
    View::Box {
        children,
        layout: Box::new(view::BoxStyle {
            on_key: Some(Action {
                name: "sessions:ui".into(),
                args: json!({"handler":request,"event":"press","props":props}),
            }),
            ..Default::default()
        }),
    }
}
fn hint(cursor: &Cursor) -> &'static str {
    match cursor.sheet {
        Sheet::None if cursor.searching => {
            " type to filter · esc leave search · ↑↓ choose · ⏎ resume"
        }
        Sheet::None => " ⏎ resume  → preview  r rename  d delete  f fork  tab here/all  esc",
        Sheet::Delete(_) => " ⏎ delete permanently · esc cancel",
        Sheet::Rename { .. } => " ⏎ save title · esc cancel",
        _ => " esc back · ↑↓ choose",
    }
}
fn directory(
    rows: &[Row],
    cursor: &Cursor,
    room: usize,
    width: u16,
    props: &SessionsProps,
    request: &str,
) -> Vec<View> {
    if rows.is_empty() {
        return vec![dim(" no sessions match")];
    }
    let mut lines = Vec::new();
    let mut selected_line = 0;
    for row in rows {
        let selected = cursor.selected.as_ref() == Some(&row.target);
        if selected {
            selected_line = lines.len();
        }
        let view = match &cursor.sheet {
            Sheet::Rename { id, text } if row.target == Target::Session(id.clone()) => {
                View::Input {
                    input: Box::new(view::InputProps {
                        key: Some(format!("rename:{id}")),
                        label: Some("❯ title".into()),
                        value: Some(text.clone()),
                        ..Default::default()
                    }),
                }
            }
            _ => row_view(row, selected, width, props, request),
        };
        lines.push(View::Box {
            children: vec![view],
            layout: Box::new(view::BoxStyle {
                key: Some(format!("{:?}", row.target)),
                ..Default::default()
            }),
        });
        if let Some(source) = &row.provenance {
            lines.push(dim(format!("     {source}")));
        }
        if let Some(ask) = &row.first_ask {
            lines.push(dim(format!("     {}\"{ask}\"", "  ".repeat(row.depth))));
        }
    }
    let start = selected_line
        .saturating_sub(room / 2)
        .min(lines.len().saturating_sub(room));
    lines.into_iter().skip(start).take(room).collect()
}
fn confirmation(doomed: &[crate::verbs::Deletion], room: usize) -> Vec<View> {
    let mut lines = vec![View::text(format!(
        " Delete {} session(s), children first?",
        doomed.len()
    ))];
    lines.extend(doomed.iter().take(room.saturating_sub(2)).map(|row| {
        dim(format!(
            "   {}{}",
            row.label,
            row.blocked
                .as_ref()
                .map(|reason| format!(" · {reason}"))
                .unwrap_or_default()
        ))
    }));
    if doomed.len() > room.saturating_sub(2) {
        lines.push(dim(format!(
            "   … {} more children",
            doomed.len() - room.saturating_sub(2)
        )));
    }
    lines
}
fn row_view(row: &Row, selected: bool, width: u16, props: &SessionsProps, request: &str) -> View {
    let indent = if row.depth > 0 {
        format!("{}└ ", "  ".repeat(row.depth))
    } else {
        String::new()
    };
    pair(
        View::Button {
            children: vec![],
            button: Box::new(view::ButtonProps {
                key: Some("title".into()),
                variant: Some("plain".into()),
                on_press: Some(Action {
                    name: "sessions:ui".into(),
                    args: json!({"handler":format!("{request}:{:?}", row.target),"request":request,"event":"activate","target":row.target,"session":props.session,"action":props.action}),
                }),
                label: Some(format!(
                    "{} {indent}{}{}",
                    if selected { "❯" } else { " " },
                    row.title,
                    if row.current { " · current" } else { "" }
                )),
                ..Default::default()
            }),
        },
        dim(row.detail.clone()),
        width,
    )
}
fn pair(left: View, right: View, width: u16) -> View {
    // Ages, counts and state badges use single-cell glyphs; the surface clips the title.
    let detail = u16::try_from(right.fold().chars().count()).unwrap_or(width);
    let title = width.saturating_sub(detail).saturating_sub(1);
    View::Box {
        children: vec![
            View::Box {
                children: vec![left],
                layout: Box::new(view::BoxStyle {
                    key: Some("title-column".into()),
                    width: Some(title),
                    ..Default::default()
                }),
            },
            right,
        ],
        layout: Box::new(view::BoxStyle {
            key: Some("columns".into()),
            flex_direction: Some(view::Direction::Row),
            justify_content: Some(view::Justify::SpaceBetween),
            height: Some(1),
            ..Default::default()
        }),
    }
}
fn dim(text: impl Into<String>) -> View {
    View::Text {
        children: vec![view::TextChild::String(text.into())],
        style: view::TextStyle {
            dim_color: Some(true),
            ..Default::default()
        },
    }
}
