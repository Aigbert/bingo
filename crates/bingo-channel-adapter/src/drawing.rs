//! A checked host answer, its original request, and the adapter's final line.
use bingo_sdk::{
    Action, SessionId, View,
    component::Props,
    render::{Component, RenderRequest},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Drawing {
    pub session: SessionId,
    pub cwd: std::path::PathBuf,
    pub request: RenderRequest,
    pub view: View,
    pub footer: String,
}

impl Drawing {
    pub fn fold(&self) -> String {
        let body = fold(&self.view, &self.request);
        if self.footer.is_empty() {
            body
        } else {
            format!("{body}\n\n---\n{}", self.footer)
        }
    }

    /// Only a handler in the checked answer can cross the input boundary,
    /// and only one that goes back to the drawing it was pressed on: a
    /// control naming any other command is not the presser's to send.
    pub fn permits(&self, action: &Action) -> bool {
        bingo_sdk::CommandSpec::is_ui(&action.name) && permits(&self.view, action)
    }
}

pub fn engine(component: Component, props: &Value, original: &RenderRequest) -> String {
    let parsed = Props::parse(component, props.clone())
        .ok()
        .filter(|p| p.component() == original.component)
        .or_else(|| Props::parse(original.component, original.props.clone()).ok());
    match parsed {
        Some(Props::AssistantMessage(p)) => p.text,
        Some(Props::CommandOutput(p)) => p.text,
        Some(Props::InfoNotice(p)) => p.text,
        Some(Props::ShellRun(p)) => bingo_sdk::shell_note(&p.command, &p.output, p.exit),
        _ => String::new(),
    }
}

fn fold(view: &View, original: &RenderRequest) -> String {
    match view {
        View::Engine { component, props } => engine(*component, props, original),
        View::Box { children, .. } => children
            .iter()
            .map(|v| fold(v, original))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => view.fold_markdown(),
    }
}

fn permits(view: &View, action: &Action) -> bool {
    match view {
        View::Box { children, .. } => children.iter().any(|v| permits(v, action)),
        View::Button { button, .. } => button.on_press.as_ref() == Some(action),
        View::Select { select } => select.on_select.as_ref() == Some(action),
        View::Input { input } => input.on_submit.as_ref() == Some(action),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::render::{SurfaceName, Viewport};
    use serde_json::json;
    #[test]
    fn only_native_controls_in_a_checked_answer_authorize_actions() {
        let action = Action {
            name: "mod:ui".into(),
            args: json!({"handler":"one"}),
        };
        let mut drawing = Drawing {
            session: SessionId::from_raw("s"),
            cwd: "/work".into(),
            request: RenderRequest {
                component: Component::InfoNotice,
                request_id: "r".into(),
                surface: SurfaceName::Chat,
                props: json!({"text":"own","command":null}),
                viewport: Viewport {
                    glyphs: Default::default(),
                    columns: 0,
                    rows: 0,
                    place: None,
                },
            },
            view: serde_json::from_value(
                json!({"kind":"Button","children":["Run"],"onPress":action}),
            )
            .unwrap(),
            footer: String::new(),
        };
        assert!(drawing.permits(&action));
        assert!(!drawing.permits(&Action {
            name: "forged".into(),
            ..action.clone()
        }));
        let typed = Action {
            name: "plugins".into(),
            args: json!("trust evil full"),
        };
        drawing.view =
            serde_json::from_value(json!({"kind":"Button","children":["OK"],"onPress":typed}))
                .unwrap();
        assert!(
            !drawing.permits(&typed),
            "a drawn control naming another command is never sent"
        );
        drawing.view = View::text(serde_json::to_string(&action).unwrap());
        assert!(!drawing.permits(&action));
        drawing.view = drawing.request.engine();
        assert_eq!(drawing.fold(), "own");
    }
}
