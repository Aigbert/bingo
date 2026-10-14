use bingo_sdk::*;
use serde_json::{Value, json};

pub fn tree() -> View {
    View::Box {
        children: vec![View::text("Hello from a built-in mod")],
        layout: Box::new(view::BoxStyle {
            on_key: Some(Action {
                name: "hello:ui".into(),
                args: json!({"handler":"keys", "event":"press"}),
            }),
            ..Default::default()
        }),
    }
}

pub struct Drawing;
#[async_trait::async_trait]
impl Interceptor for Drawing {
    fn id(&self) -> &str {
        "hello"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"component":"Toast"}),
            ..LinkMatcher::at(catalogue::UI_RENDER)
        }
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        _: &HookContext,
        _: Value,
        _: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        serde_json::to_value(tree())
            .map_err(|error| KernelError::new(ErrorCode::Internal, error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tree_uses_only_shared_elements_and_owned_handlers() {
        assert_eq!(tree().fold(), "Hello from a built-in mod");
        assert!(
            view::answer_ok(render::Component::Toast, &json!({}), "hello", None, &tree()).is_ok()
        );
    }
}
