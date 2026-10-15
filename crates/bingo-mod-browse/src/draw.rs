use crate::{Cursor, Shared};
use bingo_sdk::{component::BrowseProps, *};
use serde_json::{Value, json};
pub(crate) struct Drawing(pub Shared);
#[async_trait::async_trait]
impl Interceptor for Drawing {
    fn id(&self) -> &str {
        "browse"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"component":"Browse"}),
            ..LinkMatcher::at(catalogue::UI_RENDER)
        }
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        cx: &HookContext,
        value: Value,
        _: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let request: render::RenderRequest = serde_json::from_value(value).map_err(crate::error)?;
        let mut props: BrowseProps = serde_json::from_value(request.props).map_err(crate::error)?;
        let id = (cx.session.clone(), request.request_id.clone());
        let mut cursor = self
            .0
            .lock()
            .map_err(crate::error)?
            .get(&id)
            .cloned()
            .unwrap_or_else(|| crate::handler::initial(&props));
        if !props
            .turns
            .iter()
            .any(|turn| Some(&turn.id) == cursor.selected.as_ref())
        {
            cursor.selected = props.turns.last().map(|turn| turn.id.clone());
            cursor.sheet = None;
        }
        if cursor.sheet.is_none() && props.query == cursor.query {
            cursor.current = cursor.current.min(props.count.saturating_sub(1));
            if cursor.current == props.current
                && let Some(hit) = &props.hit
            {
                cursor.selected = hit.turn.clone().or(cursor.selected);
            }
        }
        props.selected = cursor.selected.clone();
        props.query = cursor.query.clone();
        props.current = cursor.current;
        props.refresh = cursor
            .sheet
            .as_ref()
            .is_some_and(|sheet| sheet.length != props.length);
        props.previewed = cursor.sheet.is_some() && !props.refresh;
        let view = tree(&props, &cursor, &request.request_id, request.viewport.rows);
        self.0.lock().map_err(crate::error)?.insert(id, cursor);
        serde_json::to_value(view).map_err(crate::error)
    }
}
fn tree(props: &BrowseProps, cursor: &Cursor, request: &str, height: u16) -> View {
    let at = props
        .turns
        .iter()
        .position(|t| Some(&t.id) == props.selected.as_ref());
    let query = cursor.editing.as_ref().unwrap_or(&props.query);
    let hint = if cursor.editing.is_some() {
        "⏎ search · esc clear"
    } else {
        "[ ] turn  ⏎ page  y copy  r rewind"
    };
    let position = format!(
        "browse · turn {} of {}",
        at.map_or(0, |a| a + 1),
        props.turns.len()
    );
    let search = if query.is_empty() {
        String::new()
    } else {
        format!(
            " · {query:?} {}/{}{} (n/N)",
            if props.count == 0 {
                0
            } else {
                props.current + 1
            },
            props.count,
            if props.count == 1000 { "+" } else { "" }
        )
    };
    let mut children = vec![View::text(format!("{position}{search}  {hint}"))];
    if let Some(shown) = &cursor.sheet
        && let Some(at) = at
        && !props.refresh
    {
        children.push(sheet(cursor, shown, at, height));
    }
    View::Box {
        children,
        layout: Box::new(view::BoxStyle {
            on_key: Some(Action {
                name: "browse:ui".into(),
                args: json!({"handler":request,"event":"press","props":props}),
            }),
            ..Default::default()
        }),
    }
}
fn sheet(cursor: &Cursor, shown: &crate::Preview, at: usize, height: u16) -> View {
    let Some(sheet) = crate::sheet::sheet(&shown.turns, &shown.touched, cursor.mode) else {
        return View::text("no turn to rewind");
    };
    let range = if shown.turns.len() == 1 {
        format!("turn {}", at + 1)
    } else {
        format!("turns {}–{}", at + 1, at + shown.turns.len())
    };
    let summary = if cursor.mode == crate::sheet::Mode::Files {
        "no turns are undone".into()
    } else {
        format!(
            "undoes {range} · {} items",
            shown.turns.last().map_or(0, |t| t.last + 1) - shown.turns[0].first
        )
    };
    let mut children = vec![
        View::text(format!("rewind to turn {}", at + 1)),
        View::text(summary),
    ];
    file_rows(&mut children, &sheet, shown, cursor, height);
    children.push(View::text(format!(
        "{}    ⏎ rewind  esc",
        [
            crate::sheet::Mode::Both,
            crate::sheet::Mode::Conversation,
            crate::sheet::Mode::Files
        ]
        .map(|m| format!("{}{}", if m == cursor.mode { "❯ " } else { "" }, m.name()))
        .join("    ")
    )));
    View::Box {
        children,
        layout: Box::new(view::BoxStyle {
            border_style: Some("single".into()),
            spacing: view::Spacing {
                padding_x: Some(1),
                ..Default::default()
            },
            ..Default::default()
        }),
    }
}

fn file_rows(
    children: &mut Vec<View>,
    sheet: &crate::sheet::Sheet,
    shown: &crate::Preview,
    cursor: &Cursor,
    height: u16,
) {
    let files = if cursor.mode == crate::sheet::Mode::Conversation {
        1
    } else {
        shown.touched.files.len().max(1)
    };
    let room = usize::from(height).saturating_sub(9).max(1);
    children.extend(
        sheet
            .rows
            .iter()
            .skip(cursor.sheet_top)
            .take(files.saturating_sub(cursor.sheet_top).min(room))
            .map(View::text),
    );
    if files > room {
        children.push(View::text(format!(
            "↑↓ files {}–{} of {files}",
            cursor.sheet_top + 1,
            (cursor.sheet_top + room).min(files)
        )));
    }
    children.extend(sheet.rows.iter().skip(files).map(View::text));
}
