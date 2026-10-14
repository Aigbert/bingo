//! The headless terminal client uses the same render and action doors.
use bingo_sdk::*;
use serde_json::Value;
pub fn find<'a>(tree: &'a View, key: &str) -> Option<&'a View> {
    nodes(tree).into_iter().rev().find(|view| {
        let keyed = match view {
            View::Button { button, .. } => {
                button.key.as_deref() == Some(key)
                    || button
                        .hotkey
                        .is_some_and(|hotkey| hotkey.to_string() == key)
            }
            View::Input { input } => input.key.as_deref() == Some(key),
            View::Select { select } => select.key.as_deref() == Some(key),
            _ => false,
        };
        keyed || view.fold().contains(key)
    })
}
pub fn action(tree: &View, key: &str, event: &str, value: Value) -> Result<Action, String> {
    let view = find(tree, key).ok_or_else(|| format!("no element matches {key:?}"))?;
    let mut action = handlers(view)
        .into_iter()
        .find(|action| action.args["event"] == event)
        .cloned()
        .ok_or_else(|| format!("{key:?} has no {event} handler"))?;
    action.args["value"] = value;
    Ok(action)
}

pub struct Mount {
    pub request: render::RenderRequest,
    tree: Option<View>,
}
impl Mount {
    pub fn new(component: Value, props: Value, viewport: Value) -> Result<Self, String> {
        let pane = component.get("pane").and_then(Value::as_str);
        let request = serde_json::from_value(serde_json::json!({
            "component":if pane.is_some() { Value::String("Pane".into()) } else { component.clone() },
            "requestId":pane.map(str::to_string).unwrap_or_else(|| ItemId::mint().to_string()),
            "surface":"terminal", "props":props,
            "viewport":{"columns":viewport.get("columns").cloned().unwrap_or(serde_json::json!(80)),"rows":viewport.get("rows").cloned().unwrap_or(serde_json::json!(24))}
        })).map_err(|e| e.to_string())?;
        Ok(Self {
            request,
            tree: None,
        })
    }
    pub fn invalidate(&mut self, requests: &[String]) {
        if requests.contains(&self.request.request_id) {
            self.tree = None;
        }
    }
    pub fn resize(&mut self, columns: u16, rows: u16) {
        if self.request.viewport.columns != columns {
            self.tree = None;
        }
        self.request.viewport.columns = columns;
        self.request.viewport.rows = rows;
    }
    pub async fn tree(&mut self, host: &HostHandle, session: &SessionId) -> Result<&View, String> {
        if self.tree.is_none() {
            self.tree = Some(
                host.render(session, self.request.clone())
                    .await
                    .map_err(|e| e.to_string())?,
            );
        }
        self.tree
            .as_ref()
            .ok_or_else(|| "render did not return a tree".into())
    }
}
fn nodes(root: &View) -> Vec<&View> {
    let mut pending = vec![root];
    let mut found = Vec::new();
    while let Some(view) = pending.pop() {
        match view {
            View::Box { children, .. } => pending.extend(children.iter().rev()),
            View::Panel { child, .. } => pending.push(child),
            View::Text { children, .. } | View::Button { children, .. } => {
                pending.extend(children.iter().rev().filter_map(|child| match child {
                    bingo_sdk::view::TextChild::Node(view) => Some(view.as_ref()),
                    _ => None,
                }))
            }
            _ => (),
        }
        found.push(view);
        if found.len() > 4096 {
            break;
        }
    }
    found
}
fn handlers(view: &View) -> Vec<&Action> {
    match view {
        View::Button { button, .. } => button.on_press.iter().collect(),
        View::Input { input } => input
            .on_input
            .iter()
            .chain(input.on_submit.iter())
            .collect(),
        View::Select { select } => select.on_select.iter().collect(),
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn interaction_keeps_the_minted_action_and_sets_only_its_value() {
        let tree: View = serde_json::from_value(json!({"kind":"Input","key":"name","onInput":{"name":"counter:ui","args":{"handler":"h","event":"input"}}})).unwrap();
        let fired = action(&tree, "name", "input", json!("Ada")).unwrap();
        assert_eq!(fired.name, "counter:ui");
        assert_eq!(
            fired.args,
            json!({"handler":"h","event":"input","value":"Ada"})
        );
        assert!(action(&tree, "missing", "input", json!("Ada")).is_err());
        assert!(action(&tree, "name", "press", Value::Null).is_err());
    }
}
