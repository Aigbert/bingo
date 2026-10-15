use crate::{
    ID, REQUEST, Selections, error,
    rows::{Row, Tone},
};
use bingo_sdk::view::{BoxStyle, Direction, TextChild, TextStyle};
use bingo_sdk::*;
use serde_json::{Value, json};
use std::time::Instant;

pub struct Drawing {
    pub(crate) selections: Selections,
    pub(crate) started: Instant,
}
#[async_trait::async_trait]
impl Interceptor for Drawing {
    fn id(&self) -> &str {
        ID
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"component":"Status"}),
            ..LinkMatcher::at(catalogue::UI_RENDER)
        }
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        cx: &HookContext,
        input: Value,
        _: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let request: render::RenderRequest = serde_json::from_value(input).map_err(error)?;
        let props: component::StatusProps = serde_json::from_value(request.props).map_err(error)?;
        let facts = match props.facts {
            Some(facts) => facts,
            None => (&crate::snapshot(&cx.host, &cx.session).await?.snapshot).into(),
        };
        let rows = read(&cx.host, &facts).await?;
        let selected = {
            let mut selections = self.selections.lock().map_err(error)?;
            let selection = selections
                .entry(cx.session.clone())
                .or_insert_with(|| props.selected.as_deref().unwrap_or("session").into());
            selection.seen = Instant::now();
            selection.label.clone()
        };
        let tree = tree(
            &rows,
            &selected,
            request.viewport.columns,
            request.viewport.rows,
            self.started.elapsed().as_secs(),
        );
        serde_json::to_value(tree).map_err(error)
    }
}
pub(crate) async fn read(
    host: &HostHandle,
    state: &component::StatusFacts,
) -> Result<Vec<Row>, KernelError> {
    let plugins = host.catalog(CatalogKind::Plugins).await?;
    let models = host.catalog(CatalogKind::Models).await?;
    let id = format!(
        "{}/{}",
        state.summary.provider.as_deref().unwrap_or_default(),
        state.summary.model.as_deref().unwrap_or_default()
    );
    let price = models
        .entries
        .iter()
        .find(|e| e.id == id)
        .and_then(|e| serde_json::from_value(e.meta["price"].clone()).ok());
    let servers = match bingo_services::mcp::find(host) {
        Some(service) => service.rows().await,
        None => vec![],
    };
    let mut rows = crate::rows::rows(state, &plugins.entries, &servers, price);
    let pending = match bingo_services::schedule::find(host) {
        Some(service) => service.pending(state.summary.id.clone()).await?,
        None => None,
    };
    if let Some(wake) = crate::rows::wake(pending.as_ref(), jiff::Timestamp::now()) {
        rows.insert(3, wake);
    }
    Ok(rows)
}
pub fn tree(rows: &[Row], selected: &str, columns: u16, height: u16, uptime: u64) -> View {
    let selected = crate::keys::selected(rows, selected);
    let mut children = vec![heading(columns, uptime)];
    let budget = usize::from(height.saturating_sub(1)).max(1);
    let start = start(rows, selected, budget);
    let mut used = 0;
    for (index, row) in rows.iter().enumerate().skip(start) {
        if used >= budget {
            break;
        }
        children.push(draw_row(row, index == selected, columns));
        used += 1;
        if let Some(detail) = &row.detail
            && used < budget
        {
            children.push(text(&format!("               {detail}"), Tone::Dim));
            used += 1;
        }
    }
    View::Box {
        children,
        layout: Box::new(BoxStyle {
            on_key: Some(Action {
                name: format!("{ID}:ui"),
                args: json!({"handler":REQUEST,"event":"press","selected":rows.get(selected).map(|row| &row.label)}),
            }),
            ..Default::default()
        }),
    }
}
fn heading(columns: u16, uptime: u64) -> View {
    pair(
        vec![
            text(" status", Tone::Ok),
            text("   live; ⏎ on a row goes there", Tone::Dim),
        ],
        &format!("bingo {} · up {}m ", env!("CARGO_PKG_VERSION"), uptime / 60),
        columns,
    )
}
fn hint(row: &Row) -> &str {
    match row.label.as_str() {
        "session" => "rename",
        "model" => "change",
        "context" => "compact now",
        "permission" => "change mode",
        "plugins" => "/plugins",
        "mcp" => "/mcp",
        "wake" => "/wake off",
        _ => "revoke",
    }
}
fn start(rows: &[Row], selected: usize, budget: usize) -> usize {
    if rows.is_empty() {
        return 0;
    }
    let mut start = 0;
    while rows[start..=selected]
        .iter()
        .map(|r| 1 + usize::from(r.detail.is_some()))
        .sum::<usize>()
        > budget
        && start < selected
    {
        start += 1;
    }
    start
}
fn draw_row(row: &Row, selected: bool, columns: u16) -> View {
    let rule = row.label.starts_with("allow ");
    let label = if rule { "  allow" } else { &row.label };
    let prefix = format!(" {} {:11}", if selected { "❯" } else { " " }, label);
    let mut spans = vec![text(&prefix, if selected { Tone::Ok } else { Tone::Dim })];
    spans.extend(row.value.iter().map(|span| text(&span.text, span.tone)));
    if selected {
        pair(spans, &format!("  ⏎ {} ", hint(row)), columns)
    } else {
        line(spans)
    }
}
fn pair(left: Vec<View>, right: &str, columns: u16) -> View {
    // Let the surface measure and truncate Unicode cells inside the left column.
    View::Box {
        children: vec![
            View::Box {
                children: vec![line(left)],
                layout: Box::new(BoxStyle {
                    width: Some(columns.saturating_sub(right.chars().count() as u16)),
                    ..Default::default()
                }),
            },
            text(right, Tone::Dim),
        ],
        layout: Box::new(BoxStyle {
            flex_direction: Some(Direction::Row),
            height: Some(1),
            ..Default::default()
        }),
    }
}
fn line(children: Vec<View>) -> View {
    View::Text {
        children: children
            .into_iter()
            .map(|view| TextChild::Node(Box::new(view)))
            .collect(),
        style: TextStyle {
            wrap: Some("truncate".into()),
            ..Default::default()
        },
    }
}
fn text(value: &str, tone: Tone) -> View {
    View::Text {
        children: vec![TextChild::String(value.into())],
        style: TextStyle {
            color: match tone {
                Tone::Warn => Some("yellow".into()),
                Tone::Bad => Some("red".into()),
                _ => None,
            },
            dim_color: (tone == Tone::Dim).then_some(true),
            ..Default::default()
        },
    }
}
