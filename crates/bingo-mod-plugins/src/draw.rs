use crate::{
    Cursor, Shared,
    groups::{self, Kind},
};
use bingo_sdk::{component::PluginsProps, *};
use bingo_services::mcp::ServerRow;
use serde_json::{Value, json};
pub(crate) struct Drawing(pub Shared);
#[async_trait::async_trait]
impl Interceptor for Drawing {
    fn id(&self) -> &str {
        "plugins"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"component":"Plugins"}),
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
        let props: PluginsProps = serde_json::from_value(request.props).map_err(crate::error)?;
        let servers = crate::servers(&cx.host).await;
        let mut state = self.0.lock().map_err(crate::error)?;
        let cursor = state
            .entry((cx.session.clone(), request.request_id.clone()))
            .or_default();
        serde_json::to_value(tree(
            &props,
            &servers,
            cursor,
            &request.request_id,
            request.viewport.columns,
            request.viewport.rows,
            request.viewport.glyphs,
        ))
        .map_err(crate::error)
    }
}
pub fn tree(
    props: &PluginsProps,
    servers: &[ServerRow],
    cursor: &mut Cursor,
    request: &str,
    width: u16,
    height: u16,
    glyphs: bingo_sdk::glyph::GlyphSet,
) -> View {
    use bingo_sdk::{pick, sheet::Sheet};
    let groups = groups::groups(props.open, &props.plugins, servers, true);
    let selected = groups::selected(&groups, cursor).cloned();
    let rows: Vec<_> = groups
        .iter()
        .flat_map(|g| &g.rows)
        .map(|row| row_view(row, props, servers, &cursor.selected, glyphs))
        .collect();
    let total = rows.len();
    let on = on_count(&groups, props, servers);
    let review = review(selected.as_ref().map(|r| &r.kind), props, servers, cursor);
    let reviewing = !review.is_empty();
    let review_ready = review_ready(&review, width, height);
    let rows = if reviewing {
        review
            .into_iter()
            .enumerate()
            .map(|(i, view)| pick::Row::new(i.to_string(), view))
            .collect()
    } else {
        rows
    };
    let mut view = Sheet {
        head: "Plugins".into(),
        fact: Some(pick::text(format!("{on} of {total} on"), "dim")),
        top: vec![],
        rows,
        selected: Some(cursor.selected.clone()),
        foot: vec![],
        pick: !reviewing,
    }
    .with_limit(width, height, if reviewing { 40 } else { 8 }, glyphs);
    if let View::Box { layout, .. } = &mut view {
        layout.hints = Some(hints(
            selected.as_ref(),
            servers,
            cursor,
            reviewing,
            review_ready,
        ));
        layout.on_key = Some(Action {
            name: "plugins:ui".into(),
            args: json!({"handler":request,"event":"press","row":cursor.selected,"reviewReady":review_ready,"props":{"open":props.open,"session":props.session,"plugins":[],"action":props.action}}),
        });
    }
    if reviewing {
        view
    } else {
        bingo_sdk::sheet::clicks(view)
    }
}
fn on_count(groups: &[groups::Group], props: &PluginsProps, servers: &[ServerRow]) -> usize {
    groups
        .iter()
        .flat_map(|g| &g.rows)
        .filter(|row| match row.kind {
            Kind::Plugin(i) => groups::state(&props.plugins[i]).is_some_and(|s| s.is_on()),
            Kind::Server(i) => !matches!(
                servers[i].status,
                bingo_services::mcp::Status::Disabled | bingo_services::mcp::Status::Waiting { .. }
            ),
            Kind::Builtins => true,
        })
        .count()
}
fn review_ready(review: &[View], width: u16, height: u16) -> bool {
    review.is_empty()
        || review.len() <= bingo_sdk::sheet::capacity(height, 0, 40)
            && review.iter().all(|view| {
                pick::width(view) <= usize::from(width.saturating_sub(sheet::CONTENT_GUTTERS))
            })
}
fn hints(
    selected: Option<&groups::Row>,
    servers: &[ServerRow],
    cursor: &Cursor,
    reviewing: bool,
    review_ready: bool,
) -> component::SiteHints {
    let mut keys = vec![
        component::KeyPair::new("↑↓", "plugin"),
        component::KeyPair::new("space", "switch"),
    ];
    if matches!(selected.as_ref().map(|r| &r.kind), Some(Kind::Server(i)) if matches!(servers[*i].status, bingo_services::mcp::Status::NeedsAuth { .. }))
    {
        keys.push(component::KeyPair::new("⏎", "sign in"));
    }
    if reviewing {
        keys.clear();
        if cursor.trust.is_some() {
            keys.push(component::KeyPair::new("↑↓", "trust"));
        }
        if review_ready {
            keys.push(component::KeyPair::new("⏎", "approve"));
        }
    }
    keys.push(component::KeyPair::new(
        "esc",
        if reviewing { "cancel" } else { "close" },
    ));
    component::SiteHints::new(keys)
}
fn review(
    kind: Option<&Kind>,
    props: &PluginsProps,
    servers: &[ServerRow],
    cursor: &Cursor,
) -> Vec<View> {
    let facts = match kind {
        Some(Kind::Plugin(i)) if cursor.trust.is_some() => {
            crate::cards::plugin_card(&props.plugins[*i], &props.plugins, cursor.trust.as_ref())
        }
        Some(Kind::Server(i)) if cursor.reviewing => crate::cards::server_card(&servers[*i], true),
        _ => return vec![],
    };
    facts
        .into_iter()
        .filter(|fact| !fact.text.trim().is_empty())
        .map(|fact| pick::text(fact.text, if fact.error { "amber" } else { "dim" }))
        .collect()
}
fn row_facts<'a>(
    row: &groups::Row,
    props: &'a PluginsProps,
    servers: &[ServerRow],
) -> (bool, &'a str, &'static str, Option<String>) {
    match row.kind {
        Kind::Plugin(i) => {
            let plugin = &props.plugins[i];
            let state = groups::state(plugin);
            let error = match &state {
                Some(PluginState::Failed { why }) => Some(why.clone()),
                _ => None,
            };
            (
                state.is_some_and(|s| s.is_on()),
                plugin.meta["description"].as_str().unwrap_or_default(),
                if groups::builtin(plugin) {
                    "built in"
                } else {
                    "plugin"
                },
                error,
            )
        }
        Kind::Server(i) => {
            use bingo_services::mcp::Status;
            let server = &servers[i];
            let error = match &server.status {
                Status::Failed { why } | Status::NeedsAuth { why } => Some(why.clone()),
                _ => None,
            };
            (
                !matches!(server.status, Status::Disabled | Status::Waiting { .. }),
                props
                    .plugins
                    .iter()
                    .find(|plugin| plugin.id == format!("mcp:{}", server.name))
                    .and_then(|plugin| plugin.meta["description"].as_str())
                    .unwrap_or_default(),
                "MCP",
                error,
            )
        }
        Kind::Builtins => (true, "", "built in", None),
    }
}
fn row_view(
    row: &groups::Row,
    props: &PluginsProps,
    servers: &[ServerRow],
    selected: &str,
    glyphs: glyph::GlyphSet,
) -> pick::Row {
    use bingo_sdk::pick::{bold, line, text};
    let (on, description, kind, error) = row_facts(row, props, servers);
    let name = &row.label;
    let left = line([
        text(
            format!(
                "{}  ",
                if on {
                    glyph::Glyph::Ok
                } else {
                    glyph::Glyph::Dot
                }
                .spell(glyphs)
            ),
            if on { "good" } else { "dim" },
        ),
        if row.key == selected {
            pick::pad(bold(name, "fg"), 13)
        } else {
            pick::pad(text(name, if on { "muted" } else { "dim" }), 13)
        },
        text(description, "dim"),
    ]);
    pick::Row::new(&row.key, left).right(match error {
        Some(error) => text(error.lines().next().unwrap_or_default(), "amber"),
        None => text(kind, "faint"),
    })
}
pub fn cut(line: &str, width: usize, glyphs: glyph::GlyphSet) -> String {
    if pick::width(&pick::text(line, "fg")) <= width {
        return line.into();
    }
    let ellipsis = glyph::Glyph::Ell.spell(glyphs);
    let room = width.saturating_sub(pick::width(&pick::text(ellipsis, "fg")));
    let mut result = String::new();
    for ch in line.chars() {
        let next = format!("{result}{ch}");
        if pick::width(&pick::text(&next, "fg")) > room {
            break;
        }
        result = next;
    }
    result.push_str(ellipsis);
    result
}
#[cfg(test)]
mod sheet_tests {
    use super::*;
    #[test]
    fn cut_measures_cells_and_spells_the_ellipsis() {
        assert_eq!(
            cut("界界界界", 5, bingo_sdk::glyph::GlyphSet::Unicode),
            "界界…"
        );
        assert_eq!(cut("abcdef", 5, bingo_sdk::glyph::GlyphSet::Ascii), "ab...");
    }
    #[test]
    fn plugins_use_one_window_and_put_errors_first() {
        let plugins = (0..12).map(|i| CatalogEntry { id: format!("plugin-{i}"), label: "description".into(), meta: json!({"state":if i == 11 { json!({"kind":"failed","why":"token expired"}) } else { json!({"kind":"on"}) }}) }).collect();
        let props = PluginsProps {
            open: component::PluginsOpen::Plugins,
            session: SessionId::from_raw("s"),
            plugins,
            action: Action {
                name: "test".into(),
                args: Value::Null,
            },
        };
        let mut cursor = Cursor::default();
        let view = tree(
            &props,
            &[],
            &mut cursor,
            "test",
            100,
            18,
            Default::default(),
        );
        assert!(view.fold().contains("8 of 12"));
        assert_eq!(cursor.selected, "p:plugin-11");
        assert!(!view.fold().contains("description"));
        assert!(
            serde_json::to_string(&view)
                .unwrap()
                .contains("\"ground\":\"pick\"")
        );
    }
}
