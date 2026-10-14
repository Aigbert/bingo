//! A drawing owns handlers; a gesture owns a stable address and its value.
//!
//! An address made of keys names the same control in every drawing. A step
//! made of a sibling index does not: a re-render that shifts the siblings puts
//! another control there. So a control reached by an index carries what it
//! looked like, and only a control that still looks that way takes the
//! gesture. One that nothing names is never bound, and goes as drawn.
use bingo_sdk::{Action, View};

#[derive(Debug)]
pub struct Control {
    path: String,
    event: String,
    /// `None` when every step of the address is a key.
    identity: Option<Identity>,
}

/// What an index-addressed control showed a person.
#[derive(Debug, PartialEq)]
struct Identity {
    kind: &'static str,
    label: String,
}

struct Addressed<'a> {
    path: String,
    keyed: bool,
    node: &'a View,
}

impl Control {
    pub fn bind(view: &View, action: &Action) -> Option<Self> {
        let event = action.args["event"].as_str()?;
        let nodes = addressed(view);
        let found = nodes.iter().find(|at| drew(at.node, action))?;
        let identity = match found.keyed {
            true => None,
            false => Some(identity(found.node)?),
        };
        only(&nodes, &found.path)?;
        Some(Self {
            path: found.path.clone(),
            event: event.into(),
            identity,
        })
    }

    pub fn resolve(&self, view: &View) -> Option<Action> {
        let nodes = addressed(view);
        let node = only(&nodes, &self.path)?.node;
        if let Some(was) = &self.identity
            && identity(node).as_ref() != Some(was)
        {
            return None;
        }
        handlers(node)
            .into_iter()
            .find(|action| action.args["event"] == self.event)
            .cloned()
    }
}

/// Whether this node drew the gesture's handler.
fn drew(node: &View, action: &Action) -> bool {
    handlers(node).into_iter().any(|candidate| {
        candidate.name == action.name
            && candidate.args.get("handler").is_some()
            && candidate.args.get("handler") == action.args.get("handler")
            && candidate.args["event"] == action.args["event"]
    })
}

/// The one node at an address; siblings sharing a key name none of them.
fn only<'a, 'b>(nodes: &'b [Addressed<'a>], path: &str) -> Option<&'b Addressed<'a>> {
    let mut at = nodes.iter().filter(|node| node.path == path);
    let first = at.next()?;
    at.next().is_none().then_some(first)
}

fn identity(view: &View) -> Option<Identity> {
    let (kind, label) = match view {
        View::Button { button, children } => (
            "Button",
            button
                .label
                .clone()
                .unwrap_or_else(|| bingo_sdk::view::text_children(children)),
        ),
        View::Input { input } => ("Input", input.label.clone().or(input.placeholder.clone())?),
        View::Select { select } => ("Select", select.label.clone()?),
        _ => return None,
    };
    (!label.is_empty()).then_some(Identity { kind, label })
}

fn key(view: &View) -> Option<&str> {
    match view {
        View::Box { layout, .. } => layout.key.as_deref(),
        View::Button { button, .. } => button.key.as_deref(),
        View::Input { input } => input.key.as_deref(),
        View::Select { select } => select.key.as_deref(),
        _ => None,
    }
}

/// A drawing has one root and a panel one child, so neither step can shift.
fn addressed(view: &View) -> Vec<Addressed<'_>> {
    let mut nodes = Vec::new();
    walk(view, child_path("", 0, view), true, &mut nodes);
    nodes
}

fn walk<'a>(view: &'a View, path: String, keyed: bool, nodes: &mut Vec<Addressed<'a>>) {
    if matches!(view, View::Box { layout, .. } if layout.display.as_deref() == Some("none")) {
        return;
    }
    match view {
        View::Box { children, .. } => {
            for (index, child) in children.iter().enumerate() {
                let keyed = keyed && key(child).is_some();
                walk(child, child_path(&path, index, child), keyed, nodes);
            }
        }
        View::Panel { child, .. } => walk(
            child,
            child_path(&format!("{path}/child"), 0, child),
            keyed,
            nodes,
        ),
        _ => {}
    }
    nodes.push(Addressed {
        path,
        keyed,
        node: view,
    });
}

fn child_path(parent: &str, index: usize, child: &View) -> String {
    match key(child) {
        Some(key) => format!("{parent}/key/{}", key.replace('~', "~0").replace('/', "~1")),
        None => format!("{parent}/children/{index}"),
    }
}

fn handlers(view: &View) -> Vec<&Action> {
    match view {
        View::Box { layout, .. } => layout.on_key.iter().collect(),
        View::Button { button, .. } => button.on_press.iter().collect(),
        View::Input { input } => input
            .on_input
            .iter()
            .chain(input.on_submit.iter())
            .collect(),
        View::Select { select } => select.on_select.iter().collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn input(key: Option<&str>, handler: &str) -> View {
        serde_json::from_value(json!({"kind":"Input","key":key,"label":"Name",
            "onInput":{"name":"counter:ui","args":{"handler":handler,"event":"input"}}}))
        .unwrap()
    }
    fn button(key: Option<&str>, label: &str, handler: &str) -> View {
        serde_json::from_value(json!({"kind":"Button","key":key,"label":label,
            "onPress":{"name":"counter:ui","args":{"handler":handler,"event":"press"}}}))
        .unwrap()
    }
    fn column(children: Vec<View>) -> View {
        View::Box {
            children,
            layout: Default::default(),
        }
    }
    fn press(view: &View) -> Action {
        handlers(view)[0].clone()
    }

    #[test]
    fn a_keyed_control_survives_sibling_moves_but_not_removal() {
        let old = column(vec![input(Some("name"), "old")]);
        let action = handlers(&input(Some("name"), "old"))[0].clone();
        let bound = Control::bind(&old, &action).unwrap();
        let next = column(vec![
            input(Some("other"), "other"),
            input(Some("name"), "new"),
        ]);
        assert_eq!(bound.resolve(&next).unwrap().args["handler"], "new");
        assert!(
            bound
                .resolve(&column(vec![input(Some("other"), "other")]))
                .is_none()
        );
    }

    #[test]
    fn an_unkeyed_control_uses_its_tree_path_and_event() {
        let old = column(vec![View::text("label"), input(None, "old")]);
        let action = handlers(&input(None, "old"))[0].clone();
        let bound = Control::bind(&old, &action).unwrap();
        let next = column(vec![View::text("new label"), input(None, "new")]);
        assert_eq!(bound.resolve(&next).unwrap().args["handler"], "new");
        assert!(bound.resolve(&column(vec![input(None, "other")])).is_none());
    }

    #[test]
    fn an_unkeyed_control_shifted_onto_another_does_not_take_its_gesture() {
        let old = column(vec![button(None, "A", "a-0"), button(None, "B", "b-0")]);
        let bound = Control::bind(&old, &press(&button(None, "B", "b-0"))).unwrap();
        let shifted = column(vec![
            button(None, "X", "x-1"),
            button(None, "A", "a-1"),
            button(None, "B", "b-1"),
        ]);
        assert!(bound.resolve(&shifted).is_none());
        let relabelled = column(vec![button(None, "A", "a-1"), button(None, "B", "b-1")]);
        assert_eq!(bound.resolve(&relabelled).unwrap().args["handler"], "b-1");
    }

    #[test]
    fn a_keyed_control_under_an_unkeyed_parent_is_checked_like_an_unkeyed_one() {
        let row = |children| View::Box {
            children,
            layout: Default::default(),
        };
        let old = column(vec![row(vec![button(Some("go"), "Go", "go-0")])]);
        let bound = Control::bind(&old, &press(&button(Some("go"), "Go", "go-0"))).unwrap();
        let shifted = column(vec![row(vec![button(Some("go"), "Stop", "stop-1")])]);
        assert!(bound.resolve(&shifted).is_none());
    }

    #[test]
    fn a_control_nothing_names_is_not_bound() {
        let unnamed: View = serde_json::from_value(json!({"kind":"Input",
            "onInput":{"name":"counter:ui","args":{"handler":"h","event":"input"}}}))
        .unwrap();
        let old = column(vec![unnamed.clone()]);
        assert!(Control::bind(&old, &press(&unnamed)).is_none());
    }

    #[test]
    fn duplicate_sibling_keys_refuse_to_rebind() {
        let old = column(vec![
            button(Some("k"), "A", "a-0"),
            button(Some("k"), "B", "b-0"),
        ]);
        assert!(Control::bind(&old, &press(&button(Some("k"), "B", "b-0"))).is_none());
        let bound = Control::bind(
            &column(vec![button(Some("k"), "A", "a-0")]),
            &press(&button(Some("k"), "A", "a-0")),
        )
        .unwrap();
        let doubled = column(vec![
            button(Some("k"), "A", "a-1"),
            button(Some("k"), "A", "a-2"),
        ]);
        assert!(bound.resolve(&doubled).is_none());
    }

    #[test]
    fn replacing_a_keyed_root_does_not_retarget_its_gesture() {
        let old = input(Some("name"), "old");
        let bound = Control::bind(&old, handlers(&old)[0]).unwrap();
        assert!(bound.resolve(&input(Some("other"), "new")).is_none());
    }
}
