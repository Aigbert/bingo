use super::*;
use crate::render::Component;

/// `inherited` is what the rest of the chain answered: a hook that passes it
/// on keeps the handlers it was given unchanged and adds only its own.
pub fn answer_ok(
    component: Component,
    props: &serde_json::Value,
    plugin: &str,
    inherited: Option<&View>,
    view: &View,
) -> Result<(), String> {
    let given = inherited.map(handlers).unwrap_or_default();
    check(view, Some((component, plugin, &given)))?;
    sealed_ok(component, props, view)
}
/// Sealed drawings keep exactly one unchanged host node, independent of handlers.
pub fn sealed_ok(
    component: Component,
    props: &serde_json::Value,
    view: &View,
) -> Result<(), String> {
    if !component.sealed() {
        return Ok(());
    }
    let engines: Vec<_> = super::walk::nodes(view)
        .into_iter()
        .filter_map(|(_, node)| {
            if let View::Engine { component, props } = node {
                Some((*component, props))
            } else {
                None
            }
        })
        .collect();
    if engines.as_slice() != [(component, props)] {
        return Err("sealed component requires exactly one unchanged Engine".into());
    }
    Ok(())
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
        if matches!(node, View::Box { layout, .. } if layout.on_key.is_some()) && !path.is_empty() {
            return Err(format!("{path}: onKey is only valid at the root"));
        }
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
        assert!(answer_ok(Component::Pane, &json!({}), "counter", None, &engine).is_ok());
        assert!(answer_ok(Component::AbovePrompt, &json!({}), "counter", None, &engine).is_err());
        assert!(published_ok(&View::column(vec![engine])).is_err());
        let button: View = serde_json::from_value(json!({"kind":"Button","onPress":{
            "name":"foreign:ui","args":{"handler":"1:1","event":"press"}}}))
        .unwrap();
        assert!(answer_ok(Component::Pane, &json!({}), "counter", None, &button).is_err());
        assert!(
            answer_ok(
                Component::Pane,
                &json!({}),
                "counter",
                Some(&button),
                &button
            )
            .is_ok()
        );
        let forged: View = serde_json::from_value(json!({"kind":"Button","onPress":{
            "name":"foreign:ui","args":{"handler":"1:2","event":"press"}}}))
        .unwrap();
        assert!(
            answer_ok(
                Component::Pane,
                &json!({}),
                "counter",
                Some(&button),
                &forged
            )
            .is_err()
        );
        assert!(
            answer_ok(
                Component::Pane,
                &json!({}),
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
        assert!(answer_ok(Component::Pane, &json!({}), "counter", None, &bad_text).is_err());
    }
}

#[cfg(test)]
mod sealed_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_one_unchanged_engine_passes_for_each_sealed_site() {
        for component in [Component::Permission, Component::SignIn] {
            let props = json!({"tool":"Write","input":{"path":"private","content":"all"}});
            let engine = View::Engine {
                component,
                props: props.clone(),
            };
            let reordered = serde_json::from_str(
                r#"{"input":{"content":"all","path":"private"},"tool":"Write"}"#,
            )
            .unwrap();
            let cases = [
                (View::text("trust me"), false),
                (View::column(vec![engine.clone(), engine.clone()]), false),
                (
                    View::Engine {
                        component,
                        props: json!({"tool":"Read"}),
                    },
                    false,
                ),
                (
                    View::Engine {
                        component: Component::Pane,
                        props: props.clone(),
                    },
                    false,
                ),
                (View::column(vec![View::text("frame"), engine]), true),
                (
                    View::Engine {
                        component,
                        props: reordered,
                    },
                    true,
                ),
            ];
            for (view, accepted) in cases {
                assert_eq!(
                    answer_ok(component, &props, "frame", None, &view).is_ok(),
                    accepted,
                    "{view:?}"
                );
            }
        }
    }
}

#[cfg(test)]
mod key_contract {
    use super::*;
    use serde_json::json;
    #[test]
    fn one_owned_key_handler_at_the_root_round_trips() {
        let wire = json!({"kind":"Box","children":[],"onKey":{"name":"owner:ui","args":{"handler":"keys","event":"press"}}});
        let view: View = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(&view).unwrap(), wire);
        assert!(answer_ok(Component::Pane, &json!({}), "owner", None, &view).is_ok());
        assert!(answer_ok(Component::Pane, &json!({}), "other", None, &view).is_err());
        assert!(published_ok(&View::column(vec![view])).is_err());
    }
}
