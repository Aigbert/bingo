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
        let value = self.press(request, key, &props, &servers)?;
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
impl Handler {
    pub(crate) fn press(
        &self,
        request: &str,
        key: &str,
        props: &PluginsProps,
        servers: &[bingo_services::mcp::ServerRow],
    ) -> Result<Value, KernelError> {
        let mut state = self.0.lock().map_err(crate::error)?;
        let identity = (props.session.clone(), request.into());
        let cursor = state.entry(identity.clone()).or_default();
        let groups = groups::groups(props.open, &props.plugins, servers, cursor.expanded);
        let selected = groups::selected(&groups, cursor).cloned();
        if key == "ctrl+c" || key == "esc" && cursor.trust.is_none() && !cursor.reviewing {
            state.remove(&identity);
            return Ok(json!({"close":true}));
        }
        if key == "esc" && cursor.reviewing {
            cursor.reviewing = false;
            return Ok(json!({}));
        }
        let Some(row) = selected else {
            return Ok(json!({}));
        };
        if let Some(choice) = &mut cursor.trust {
            let Kind::Plugin(i) = row.kind else {
                cursor.trust = None;
                return Ok(json!({}));
            };
            let result = crate::trust::press(choice, &props.plugins[i], key);
            if result.is_some() {
                cursor.trust = None;
            }
            return Ok(operation(result.unwrap_or(Verb::None)));
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
        match verb {
            Verb::Trust => {
                cursor.trust = Some(Default::default());
                Ok(json!({}))
            }
            Verb::Review => {
                cursor.reviewing = true;
                Ok(json!({}))
            }
            Verb::Fold => {
                fold(cursor, props);
                Ok(json!({}))
            }
            _ => {
                if matches!(&verb, Verb::Line(line) if line.starts_with("/mcp login ") || line.starts_with("/mcp tools "))
                {
                    state.remove(&identity);
                }
                Ok(operation(verb))
            }
        }
    }
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
