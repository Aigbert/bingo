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
        Some(Kind::Server(i)) => crate::cards::server_card(&servers[*i]),
        _ => crate::cards::builtins_card(cursor.expanded),
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
        children: vec![
            header_view(header(props, servers.len()), width),
            body,
            text(
                hints(selected.map(|r| &r.kind), props, servers, cursor),
                None,
                true,
            ),
        ],
        layout: Box::new(view::BoxStyle {
            on_key: Some(Action {
                name: "plugins:ui".into(),
                args: json!({"handler":request,"event":"press","props":{"open":props.open,"session":props.session,"plugins":[],"action":props.action}}),
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
    let room = usize::from(height);
    let start = at
        .saturating_sub(room / 2)
        .min(rows.len().saturating_sub(room));
    column(
        rows.into_iter()
            .skip(start)
            .take(room)
            .map(|row| column(vec![row], Some(1)))
            .collect(),
        Some(height),
    )
}
fn hints(
    kind: Option<&Kind>,
    props: &PluginsProps,
    servers: &[ServerRow],
    cursor: &Cursor,
) -> String {
    if let Some(choice) = &cursor.trust {
        return format!(
            " ↑↓ tier · ⏎ {} · esc cancel",
            if choice.confirm {
                "grant full"
            } else {
                "grant"
            }
        );
    }
    match kind {
        Some(Kind::Builtins) => " ↑↓ pick · ⏎ show / fold · esc close".into(),
        Some(Kind::Plugin(i))
            if matches!(
                groups::state(&props.plugins[*i]),
                PluginState::Waiting { .. }
            ) =>
        {
            " ↑↓ pick · ⏎ trust it? · esc close".into()
        }
        Some(Kind::Plugin(i)) => {
            let plugin = &props.plugins[*i];
            let enabled = match groups::state(plugin) {
                PluginState::Pending { now, .. } => !now,
                state => state.is_on(),
            };
            format!(
                " ↑↓ pick · space turn {}{} · esc close",
                if enabled { "off" } else { "on" },
                if groups::builtin(plugin) {
                    ""
                } else {
                    " · r reload · t trust"
                }
            )
        }
        Some(Kind::Server(i)) => server_hints(&servers[*i]),
        None => " esc close".into(),
    }
}
fn server_hints(server: &ServerRow) -> String {
    use bingo_services::mcp::{Auth, Status};
    let primary = match server.status {
        Status::Connected { .. } => "its tools",
        Status::NeedsAuth { .. } => "sign in",
        Status::Disabled => "turn on",
        Status::Failed { .. } => "try again",
        Status::Waiting { .. } => "approve",
        _ => "wait",
    };
    let mut parts = vec!["↑↓ pick".to_string(), format!("⏎ {primary}")];
    parts.push(format!(
        "space turn {}",
        if matches!(server.status, Status::Disabled) {
            "on"
        } else {
            "off"
        }
    ));
    if matches!(
        server.status,
        Status::Connected { .. } | Status::Failed { .. }
    ) {
        parts.push("r reconnect".into());
    }
    if server.auth == Some(Auth::SignedIn) {
        parts.push("o sign out".into());
    }
    parts.push("esc close".into());
    format!(" {}", parts.join(" · "))
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
    let card = column(
        card.into_iter()
            .map(|fact| {
                text(
                    fact.text,
                    if fact.error { Some("red") } else { None },
                    false,
                )
            })
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
                sized(card, width - width * 56 / 108 - 1),
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
    format!(
        " {title}   {} plugins · {} MCP servers · one list",
        props
            .plugins
            .iter()
            .filter(|p| !p.id.starts_with("mcp:"))
            .count(),
        servers
    )
}
