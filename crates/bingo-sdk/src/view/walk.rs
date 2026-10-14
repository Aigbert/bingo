use super::*;

pub fn nodes(root: &View) -> Vec<(String, &View)> {
    let mut pending = vec![(String::new(), root)];
    let mut found = Vec::new();
    while let Some((path, view)) = pending.pop() {
        match view {
            View::Box { children, .. } => pending.extend(
                children
                    .iter()
                    .enumerate()
                    .rev()
                    .map(|(i, v)| (format!("{path}/children/{i}"), v)),
            ),
            View::Panel { child, .. } => pending.push((format!("{path}/child"), child)),
            View::Text { children, .. } | View::Button { children, .. } => {
                pending.extend(children.iter().enumerate().rev().filter_map(
                    |(i, child)| match child {
                        TextChild::Node(view) => {
                            Some((format!("{path}/children/{i}"), view.as_ref()))
                        }
                        _ => None,
                    },
                ));
            }
            _ => {}
        }
        found.push((path, view));
        if found.len() > 4096 {
            break;
        }
    }
    found
}

pub fn handlers(view: &View) -> Vec<&Action> {
    match view {
        View::Box { layout, .. } => layout.on_key.iter().collect(),
        View::Button { button, .. } => button.on_press.iter().collect(),
        View::Input { input } => input
            .on_input
            .iter()
            .chain(input.on_submit.iter())
            .collect(),
        View::Select { select } => select.on_select.iter().collect(),
        View::Actions { items } => items.iter().map(|item| &item.action).collect(),
        _ => vec![],
    }
}
