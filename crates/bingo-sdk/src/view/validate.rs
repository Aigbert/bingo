use super::*;
use crate::render::Component;

pub fn answer_ok(_component: Component, _plugin: &str, _view: &View) -> Result<(), String> { Ok(()) }
pub fn published_ok(_view: &View) -> Result<(), String> { Ok(()) }

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn answers_reject_wrong_engines_foreign_handlers_and_oversized_trees() {
        let engine = View::Engine { component: Component::Pane, props: json!({}) };
        assert!(answer_ok(Component::Pane, "counter", &engine).is_ok());
        assert!(answer_ok(Component::AbovePrompt, "counter", &engine).is_err());
        assert!(published_ok(&View::column(vec![engine])).is_err());
        let button: View = serde_json::from_value(json!({"kind":"Button","onPress":{
            "name":"foreign:ui","args":{"handler":"1:1","event":"press"}}})).unwrap();
        assert!(answer_ok(Component::Pane, "counter", &button).is_err());
        assert!(answer_ok(Component::Pane, "counter", &View::column(vec![View::text("x");4096])).is_err());
        let bad_text = View::Text { children: vec![TextChild::Node(Box::new(View::column(vec![])))], style: Default::default() };
        assert!(answer_ok(Component::Pane, "counter", &bad_text).is_err());
    }
}
