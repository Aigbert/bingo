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
        let existing = self.0.lock().map_err(crate::error)?.get(&id).cloned();
        let mut cursor = existing
            .clone()
            .unwrap_or_else(|| crate::handler::initial(&props));
        if existing.is_none() && props.confirm {
            cursor.sheet = Some(crate::handler::preview(&cx.host, &props, &cx.cwd).await?);
        }
        if !props
            .turns
            .iter()
            .any(|t| Some(&t.id) == cursor.selected.as_ref())
        {
            cursor.selected = props.turns.last().map(|t| t.id.clone());
            cursor.sheet = None;
        }
        if props.query == cursor.query {
            cursor.current = cursor.current.min(props.matches.len().saturating_sub(1));
        }
        if props.query == cursor.query
            && let Some(hit) = props.matches.get(cursor.current)
        {
            cursor.selected = hit.turn.clone().or(cursor.selected);
        }
        props.selected = cursor.selected.clone();
        props.query = cursor.query.clone();
        props.current = cursor.current;
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
        "⏎ search · esc close"
    } else {
        "[ ] turn  ⏎ page  y copy  r rewind"
    };
    let mut children = vec![View::text(format!(
        "browse · turn {} of {} · {:?} {}/{} (n/N)  {hint}",
        at.map_or(0, |a| a + 1),
        props.turns.len(),
        query,
        if props.matches.is_empty() {
            0
        } else {
            props.current + 1
        },
        props.matches.len()
    ))];
    if let Some(touched) = &cursor.sheet
        && let Some(at) = at
    {
        children.push(sheet(props, cursor, touched, at, height));
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
fn sheet(
    props: &BrowseProps,
    cursor: &Cursor,
    touched: &bingo_services::checkpoints::Touched,
    at: usize,
    height: u16,
) -> View {
    let Some(sheet) = crate::sheet::sheet(&props.turns[at..], touched, cursor.mode) else {
        return View::text("no turn to rewind");
    };
    let mut children = vec![
        View::text(format!("rewind to turn {}", at + 1)),
        View::text(format!(
            "undoes turns {}–{} · {} items",
            at + 1,
            props.turns.len(),
            props.turns.last().map_or(0, |t| t.last + 1) - props.turns[at].first
        )),
    ];
    let files = touched.files.len().max(1);
    let room = usize::from(height).saturating_sub(9).max(1);
    children.extend(
        sheet
            .rows
            .iter()
            .skip(1 + cursor.sheet_top)
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
    children.extend(sheet.rows.iter().skip(1 + files).map(View::text));
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
