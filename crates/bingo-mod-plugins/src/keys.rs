use crate::{
    Shared,
    groups::{self, Kind},
    verbs::Verb,
};
use bingo_sdk::{component::PluginsProps, *};
use serde_json::{Value, json};
pub(crate) struct Handler(pub Shared);
#[async_trait::async_trait]
impl Command for Handler {
    fn spec(&self) -> CommandSpec {
        CommandSpec::ui("plugins")
    }
    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let args: Value = serde_json::from_str(args).map_err(crate::error)?;
        let request = args["handler"].as_str().unwrap_or_default();
        let key = args["value"]["key"].as_str().unwrap_or_default();
        if args["event"] != "press" {
            return Ok(CommandOutcome::Handled { handled: false });
        }
        let props: PluginsProps =
            serde_json::from_value(args["props"].clone()).map_err(crate::error)?;
        let plugins = cx.host.catalog(CatalogKind::Plugins).await?.entries;
        let props = PluginsProps { plugins, ..props };
        let servers = crate::servers(&cx.host).await;
        let at = Press {
            session: &cx.session,
            request,
            key,
            row: args["row"].as_str(),
            vouched: render::vouched(),
        };
        let value = self.press(&at, &props, &servers)?;
        cx.host
            .invalidate(&cx.session, vec![request.into()])
            .await?;
        let mut action = props.action;
        action.args["value"] = value;
        Ok(CommandOutcome::Surface {
            action,
            fallback: None,
        })
    }
}
/// One key press: the session the kernel says it came from, the drawing it
/// answers, the row that drawing showed selected, and whether the kernel
/// vouches that this panel drew it and the press reached it as pressed.
pub(crate) struct Press<'a> {
    pub session: &'a SessionId,
    pub request: &'a str,
    pub key: &'a str,
    pub row: Option<&'a str>,
    pub vouched: bool,
}
impl Handler {
    pub(crate) fn press(
        &self,
        at: &Press<'_>,
        props: &PluginsProps,
        servers: &[bingo_services::mcp::ServerRow],
    ) -> Result<Value, KernelError> {
        if !at.vouched {
            return Ok(unvouched());
        }
        let key = at.key;
        let mut state = self.0.lock().map_err(crate::error)?;
        let identity = (at.session.clone(), at.request.into());
        let cursor = state.entry(identity.clone()).or_default();
        if key == "ctrl+c" || key == "esc" && cursor.trust.is_none() && !cursor.reviewing {
            state.remove(&identity);
            return Ok(json!({"close":true}));
        }
        if key == "esc" && cursor.reviewing {
            cursor.reviewing = false;
            return Ok(json!({}));
        }
        let groups = groups::groups(props.open, &props.plugins, servers, cursor.expanded);
        let Some(row) = groups::current(&groups, cursor).cloned() else {
            return Ok(stale());
        };
        if !matches!(key, "up" | "down") && at.row != Some(row.key.as_str()) {
            return Ok(stale());
        }
        if cursor.trust.is_some() {
            return Ok(trusting(cursor, &row.kind, props, key));
        }
        if key == "up" || key == "down" {
            navigate(cursor, &groups, key);
            return Ok(json!({}));
        }
        let verb = match row.kind {
            Kind::Plugin(i) => crate::verbs::plugin(&props.plugins[i], key),
            Kind::Server(i) => crate::verbs::server(&servers[i], cursor.reviewing, key),
            Kind::Builtins => crate::verbs::pressed(crate::verbs::builtins(cursor.expanded), key),
        };
        cursor.reviewing = false;
        if matches!(&verb, Verb::Line(line) if line.starts_with("/mcp login ") || line.starts_with("/mcp tools "))
        {
            state.remove(&identity);
            return Ok(operation(verb));
        }
        Ok(acted(cursor, verb, props))
    }
}
/// What a verb the selected row answered does to the panel, or runs.
fn acted(cursor: &mut crate::Cursor, verb: Verb, props: &PluginsProps) -> Value {
    match verb {
        Verb::Trust => cursor.trust = Some(Default::default()),
        Verb::Review => cursor.reviewing = true,
        Verb::Fold => fold(cursor, props),
        verb => return operation(verb),
    }
    json!({})
}
/// A key while the trust choice is open: it answers the choice, which closes
/// once a tier is picked or the row is no longer a plugin.
fn trusting(cursor: &mut crate::Cursor, kind: &Kind, props: &PluginsProps, key: &str) -> Value {
    let (Kind::Plugin(i), Some(choice)) = (kind, &mut cursor.trust) else {
        cursor.trust = None;
        return json!({});
    };
    let result = crate::trust::press(choice, &props.plugins[*i], key);
    if result.is_some() {
        cursor.trust = None;
    }
    operation(result.unwrap_or(Verb::None))
}
/// A key that answers a drawing whose selected row is gone or no longer the
/// cursor's acts on nothing; the redraw shows the person what is there now.
fn stale() -> Value {
    json!({"message":"the list changed; look again and press once more"})
}
/// A key the kernel does not vouch for, drawn by someone else or changed on
/// its way here, moves no cursor, opens no review and runs nothing.
fn unvouched() -> Value {
    json!({"message":"that key did not come from this panel as pressed; nothing changed"})
}
fn operation(verb: Verb) -> Value {
    match verb {
        Verb::Line(line) => json!({"run":line}),
        Verb::Message(message) => json!({"message":message}),
        _ => json!({}),
    }
}

fn navigate(cursor: &mut crate::Cursor, groups: &[groups::Group], key: &str) {
    let rows = groups.iter().flat_map(|g| &g.rows).collect::<Vec<_>>();
    let at = rows
        .iter()
        .position(|r| r.key == cursor.selected)
        .unwrap_or(0);
    let at = if key == "up" {
        at.saturating_sub(1)
    } else {
        (at + 1).min(rows.len().saturating_sub(1))
    };
    if let Some(row) = rows.get(at) {
        cursor.selected = row.key.clone();
    }
    cursor.reviewing = false;
}

fn fold(cursor: &mut crate::Cursor, props: &PluginsProps) {
    cursor.expanded = !cursor.expanded;
    if cursor.expanded {
        cursor.selected = props
            .plugins
            .iter()
            .find(|p| groups::builtin(p))
            .map(|p| format!("p:{}", p.id))
            .unwrap_or_default();
    }
}
