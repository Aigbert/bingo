use super::*;
use crate::render::Component;

/// `inherited` is what the rest of the chain answered: a hook that passes it
/// on keeps the handlers it was given unchanged and adds only its own.
pub fn answer_ok(
    component: Component,
    plugin: &str,
    inherited: Option<&View>,
    view: &View,
) -> Result<(), String> {
    let given = inherited.map(handlers).unwrap_or_default();
    check(view, Some((component, plugin, &given)))
}
pub fn published_ok(view: &View) -> Result<(), String> {
    check(view, None)
}

fn handlers(view: &View) -> Vec<&Action> {
    super::walk::nodes(view)
        .into_iter()
        .flat_map(|(_, node)| super::walk::handlers(node))
        .collect()
}

fn check(view: &View, answer: Option<(Component, &str, &[&Action])>) -> Result<(), String> {
    let nodes = super::walk::nodes(view);
    if nodes.len() > 4096 {
        return Err("view exceeds 4096 nodes".into());
    }
    for (path, node) in nodes {
        if let View::Engine { component, .. } = node
            && answer.is_none_or(|(asked, _, _)| asked != *component)
        {
            return Err(format!(
                "{path}: Engine is only valid for the requested component"
            ));
        }
        if let Some((_, plugin, given)) = answer {
            for action in super::walk::handlers(node) {
                if action.name != format!("{plugin}:ui") && !given.contains(&action) {
                    return Err(format!("{path}: handler belongs to another command"));
                }
            }
        }
        if let View::Text { children, .. } | View::Button { children, .. } = node
            && children.iter().any(|child| matches!(child, TextChild::Node(node) if !matches!(node.as_ref(), View::Text { .. }))) {
            return Err(format!("{path}: text children must be strings or Text"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn answers_reject_wrong_engines_foreign_handlers_and_oversized_trees() {
        let engine = View::Engine {
            component: Component::Pane,
            props: json!({}),
        };
        assert!(answer_ok(Component::Pane, "counter", None, &engine).is_ok());
        assert!(answer_ok(Component::AbovePrompt, "counter", None, &engine).is_err());
        assert!(published_ok(&View::column(vec![engine])).is_err());
        let button: View = serde_json::from_value(json!({"kind":"Button","onPress":{
            "name":"foreign:ui","args":{"handler":"1:1","event":"press"}}}))
        .unwrap();
        assert!(answer_ok(Component::Pane, "counter", None, &button).is_err());
        assert!(answer_ok(Component::Pane, "counter", Some(&button), &button).is_ok());
        let forged: View = serde_json::from_value(json!({"kind":"Button","onPress":{
            "name":"foreign:ui","args":{"handler":"1:2","event":"press"}}}))
        .unwrap();
        assert!(answer_ok(Component::Pane, "counter", Some(&button), &forged).is_err());
        assert!(
            answer_ok(
                Component::Pane,
                "counter",
                None,
                &View::column(vec![View::text("x"); 4096])
            )
            .is_err()
        );
        let bad_text = View::Text {
            children: vec![TextChild::Node(Box::new(View::column(vec![])))],
            style: Default::default(),
        };
        assert!(answer_ok(Component::Pane, "counter", None, &bad_text).is_err());
    }
}
