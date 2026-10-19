use crate::{
    ID, REQUEST, Selections, error,
    rows::{Row, Tone},
};
use bingo_sdk::*;
use serde_json::{Value, json};
use std::time::Instant;

pub struct Drawing {
    pub(crate) selections: Selections,
    /// The home a directory is spelled from.
    pub(crate) home: std::path::PathBuf,
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
        let rows = read(&cx.host, &facts, &self.home, request.viewport.glyphs).await?;
        let selected = {
            let mut selections = self.selections.lock().map_err(error)?;
            let selection = selections
                .entry(cx.session.clone())
                .or_insert_with(|| props.selected.as_deref().unwrap_or("model").into());
            selection.seen = Instant::now();
            selection.label.clone()
        };
        let tree = tree_named(
            &rows,
            &selected,
            request.viewport.columns,
            request.viewport.rows,
            facts.summary.title.as_deref().unwrap_or("this session"),
            request.viewport.glyphs,
        );
        serde_json::to_value(tree).map_err(error)
    }
}
pub(crate) async fn read(
    host: &HostHandle,
    state: &component::StatusFacts,
    home: &std::path::Path,
    glyphs: glyph::GlyphSet,
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
    let around = crate::rows::Around {
        home: home.into(),
        working: working(host, &state.summary.id).await,
    };
    Ok(crate::rows::rows(
        state,
        &plugins.entries,
        &servers,
        price,
        &around,
        glyphs,
    ))
}
/// The session's agents that are not idle. A host that keeps no sessions
/// has none.
async fn working(host: &HostHandle, session: &SessionId) -> usize {
    let filter = SessionFilter {
        parent: Some(session.clone()),
        ..Default::default()
    };
    host.sessions(filter)
        .await
        .map(|agents| agents.iter().filter(|a| a.busy || a.asking).count())
        .unwrap_or_default()
}
pub fn tree(
    rows: &[Row],
    selected: &str,
    columns: u16,
    height: u16,
    glyphs: bingo_sdk::glyph::GlyphSet,
) -> View {
    tree_named(rows, selected, columns, height, "this session", glyphs)
}
fn tree_named(
    rows: &[Row],
    selected: &str,
    columns: u16,
    height: u16,
    title: &str,
    glyphs: bingo_sdk::glyph::GlyphSet,
) -> View {
    use bingo_sdk::{pick, sheet::Sheet};
    let selected = crate::keys::selected(rows, selected);
    let list = rows
        .iter()
        .map(|row| {
            let mut spans = vec![pick::pad(pick::text(&row.label, "dim"), 12)];
            spans.extend(row.value.iter().map(|span| {
                pick::text(
                    &span.text,
                    match span.tone {
                        Tone::Ok => "fg",
                        Tone::Warn => "amber",
                        Tone::Bad => "bad",
                        Tone::Dim => "dim",
                        Tone::Muted => "muted",
                        Tone::Auto => "auto",
                        Tone::Plan => "plan",
                        Tone::Glow => "glow",
                        Tone::Faint => "faint",
                        Tone::Code => "code",
                    },
                )
            }));
            pick::Row::new(&row.label, pick::line(spans))
        })
        .collect();
    let mut view = Sheet {
        head: "Status".into(),
        fact: Some(pick::text(title, "dim")),
        top: vec![],
        rows: list,
        selected: rows.get(selected).map(|r| r.label.clone()),
        foot: vec![],
        pick: true,
    }
    .view(columns, height, glyphs);
    if let View::Box { layout, .. } = &mut view {
        let mut pairs = vec![component::KeyPair::new("↑↓", "line")];
        if rows.get(selected).is_some_and(|r| r.command.is_some()) {
            pairs.push(component::KeyPair::new("⏎", "open"));
        }
        pairs.push(component::KeyPair::new("esc", "close"));
        layout.hints = Some(component::SiteHints::new(pairs));
        layout.on_key = Some(Action {
            name: format!("{ID}:ui"),
            args: json!({"handler":REQUEST,"event":"press","selected":rows.get(selected).map(|row| &row.label)}),
        });
    }
    bingo_sdk::sheet::clicks(view)
}
