use crate::{
    Cursor, Shared,
    groups::{self, Group, Kind},
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
) -> View {
    let groups = groups::groups(props.open, &props.plugins, servers, cursor.expanded);
    let selected = groups::selected(&groups, cursor);
    let mut card = match selected.map(|row| &row.kind) {
        Some(Kind::Plugin(i)) => {
            crate::cards::plugin_card(&props.plugins[*i], &props.plugins, cursor.trust.as_ref())
        }
        Some(Kind::Server(i)) => crate::cards::server_card(&servers[*i], cursor.reviewing),
        _ => crate::cards::builtins_card(&props.plugins, cursor.expanded),
    };
    if width < 100
        && cursor.trust.is_some()
        && let Some(at) = card
            .iter()
            .position(|fact| fact.text.starts_with(" trust "))
    {
        card = card.split_off(at);
    }
    let room = height
        .saturating_sub(if width < 100 { 3 } else { 2 })
        .clamp(1, 16);
    let body = body(&groups, cursor, card, width, room);
    View::Box {
        children: vec![header_view(header(props, servers.len()), width), body],
        layout: Box::new(view::BoxStyle {
            hints: Some(
                component::SiteHints::new(hints(selected.map(|r| &r.kind), props, servers, cursor))
                    .composer(
                        if props.open == component::PluginsOpen::Mcp {
                            "/mcp › plugins and MCP servers"
                        } else {
                            "/plugins › plugins and MCP servers"
                        },
                        "",
                    ),
            ),
            on_key: Some(Action {
                name: "plugins:ui".into(),
                args: json!({"handler":request,"event":"press","row":cursor.selected,"props":{"open":props.open,"session":props.session,"plugins":[],"action":props.action}}),
            }),
            ..Default::default()
        }),
    }
}
fn list(groups: &[Group], selected: &str, height: u16) -> View {
    let mut rows = vec![];
    let mut at = 0;
    for group in groups {
        rows.push(text(
            format!("   {}  {}", group.name, group.hint),
            None,
            true,
        ));
        for row in &group.rows {
            let is_selected = row.key == selected;
            if is_selected {
                at = rows.len();
            }
            rows.push(text(
                format!(
                    " {} {} {:20} {}",
                    if is_selected { "❯" } else { " " },
                    row.glyph,
                    row.label,
                    row.tag
                ),
                if row.glyph == "✗" {
                    Some("red")
                } else if is_selected {
                    Some("yellow")
                } else if row.glyph == "●" {
                    Some("green")
                } else {
                    None
                },
                false,
            ));
        }
    }
    column(
        window(rows, at, usize::from(height))
            .into_iter()
            .map(|row| column(vec![row], Some(1)))
            .collect(),
        Some(height),
    )
}
/// The rows that fit around the cursor's; when some do not, the last line
/// says how many are below, or, at the bottom, how many above.
fn window(rows: Vec<View>, at: usize, room: usize) -> Vec<View> {
    let count = rows.len();
    if count <= room {
        return rows;
    }
    let shown = room.saturating_sub(1);
    let start = at.saturating_sub(shown / 2).min(count - shown);
    let below = count - start - shown;
    let more = if below > 0 {
        format!("   ↓ {below} more")
    } else {
        format!("   ↑ {start} above")
    };
    rows.into_iter()
        .skip(start)
        .take(shown)
        .chain(std::iter::once(text(more, None, true)))
        .collect()
}
fn hints(
    kind: Option<&Kind>,
    props: &PluginsProps,
    servers: &[ServerRow],
    cursor: &Cursor,
) -> Vec<component::KeyPair> {
    if let Some(choice) = &cursor.trust {
        return vec![
            component::KeyPair::new("↑↓", "tier"),
            component::KeyPair::new(
                "⏎",
                if choice.confirm {
                    "grant full"
                } else {
                    "grant"
                },
            ),
            component::KeyPair::new("esc", "cancel"),
        ];
    }
    let bindings = match kind {
        Some(Kind::Builtins) => crate::verbs::builtins(cursor.expanded),
        Some(Kind::Plugin(i)) => crate::verbs::plugin_keys(&props.plugins[*i]),
        Some(Kind::Server(i)) => crate::verbs::server_keys(&servers[*i], cursor.reviewing),
        None => return vec![component::KeyPair::new("esc", "close")],
    };
    key_pairs(&bindings, if cursor.reviewing { "cancel" } else { "close" })
}
fn key_pairs(bindings: &[crate::verbs::Binding], esc: &str) -> Vec<component::KeyPair> {
    std::iter::once(component::KeyPair::new("↑↓", "pick"))
        .chain(bindings.iter().filter_map(|binding| {
            Some(component::KeyPair::new(
                if binding.key == "enter" {
                    "⏎"
                } else {
                    binding.key
                },
                binding.hint.as_ref()?,
            ))
        }))
        .chain(std::iter::once(component::KeyPair::new("esc", esc)))
        .collect()
}
#[cfg(test)]
pub fn footer(bindings: &[crate::verbs::Binding], esc: &str) -> String {
    format!(
        " {}",
        key_pairs(bindings, esc)
            .iter()
            .map(|p| format!("{} {}", p.keys, p.action))
            .collect::<Vec<_>>()
            .join(" · ")
    )
}
/// A line no wider than the card, its end replaced by `…` when it was.
pub fn cut(line: &str, width: usize) -> String {
    if line.chars().count() <= width {
        return line.into();
    }
    line.chars()
        .take(width.saturating_sub(1))
        .chain(std::iter::once('…'))
        .collect()
}
fn column(children: Vec<View>, height: Option<u16>) -> View {
    View::Box {
        children,
        layout: Box::new(view::BoxStyle {
            height,
            ..Default::default()
        }),
    }
}
fn sized(view: View, width: u16) -> View {
    View::Box {
        children: vec![view],
        layout: Box::new(view::BoxStyle {
            width: Some(width),
            ..Default::default()
        }),
    }
}
fn text(text: impl Into<String>, color: Option<&str>, dim: bool) -> View {
    View::Text {
        children: vec![view::TextChild::String(text.into())],
        style: view::TextStyle {
            color: color.map(str::to_string),
            dim_color: Some(dim),
            ..Default::default()
        },
    }
}

fn header_view(header: String, width: u16) -> View {
    View::Box {
        children: vec![
            text(header, None, false),
            text(" ● on   ○ off   ✗ failed   ◌ waiting", None, true),
        ],
        layout: Box::new(view::BoxStyle {
            flex_direction: Some(if width < 100 {
                view::Direction::Column
            } else {
                view::Direction::Row
            }),
            justify_content: Some(view::Justify::SpaceBetween),
            ..Default::default()
        }),
    }
}

fn body(
    groups: &[Group],
    cursor: &Cursor,
    card: Vec<crate::cards::Fact>,
    width: u16,
    room: u16,
) -> View {
    let list_height = if width < 100 {
        if cursor.trust.is_some() { 3 } else { 5 }
    } else {
        room
    };
    let list = list(groups, &cursor.selected, list_height);
    let card_width = if width < 100 {
        width
    } else {
        width - width * 56 / 108 - 1
    };
    let card = column(
        card.into_iter()
            .map(|fact| fact_view(fact, card_width))
            .collect(),
        None,
    );
    if width < 100 {
        column(
            vec![list, text("─".repeat(usize::from(width)), None, true), card],
            Some(room),
        )
    } else {
        View::Box {
            children: vec![
                sized(list, width * 56 / 108),
                sized(text(vec!["│"; usize::from(room)].join("\n"), None, true), 1),
                sized(card, card_width),
            ],
            layout: Box::new(view::BoxStyle {
                flex_direction: Some(view::Direction::Row),
                height: Some(room),
                ..Default::default()
            }),
        }
    }
}

fn header(props: &PluginsProps, servers: usize) -> String {
    let title = if props.open == component::PluginsOpen::Mcp {
        "MCP servers"
    } else {
        "plugins"
    };
    let plugins = props
        .plugins
        .iter()
        .filter(|p| !p.id.starts_with("mcp:"))
        .count();
    format!(
        " {title}   {} · {} · one list",
        counted(plugins, "plugin"),
        counted(servers, "MCP server")
    )
}

pub(crate) fn counted(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

fn fact_view(fact: crate::cards::Fact, width: u16) -> View {
    let Some(aside) = fact.aside else {
        if fact.error {
            return text(cut(&fact.text, usize::from(width)), Some("red"), false);
        }
        let width = usize::from(width);
        return text(
            crate::cards::hanging(&fact.text, width, fact.hang),
            None,
            false,
        );
    };
    View::Box {
        children: vec![
            text(fact.text, Some("yellow"), false),
            text(aside, None, true),
        ],
        layout: Box::new(view::BoxStyle {
            flex_direction: Some(view::Direction::Row),
            justify_content: Some(view::Justify::SpaceBetween),
            ..Default::default()
        }),
    }
}
