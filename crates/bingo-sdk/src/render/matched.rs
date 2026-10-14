use super::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct Matched(pub BTreeMap<Component, Vec<Value>>);

pub fn matched(config: &crate::ConfigView) -> Matched {
    serde_json::from_value(config.kernel["render"].clone()).unwrap_or_default()
}
impl Matched {
    pub fn asks(&self, component: Component, request: &RenderRequest) -> bool {
        component == request.component
            && self.0.get(&component).is_some_and(|partials| {
                let event = serde_json::to_value(request).unwrap_or_default();
                partials.iter().any(|partial| {
                    partial.is_null() || crate::catalogue::prefilter(partial, &event)
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn matched_partials_select_only_the_requested_component() {
        let config = crate::ConfigView {
            kernel: json!({"render":{"ToolUse":[{"props":{"tool":"sample"},"requestId":"call"}], "Pane":[{}]}}),
            plugins: Default::default(),
        };
        let mut request = RenderRequest {
            component: Component::ToolUse,
            request_id: "call".into(),
            surface: SurfaceName::Terminal,
            props: json!({"tool":"sample","input":{}}),
            viewport: Viewport {
                columns: 80,
                rows: 8,
            },
        };
        let selected = matched(&config);
        assert!(selected.asks(Component::ToolUse, &request));
        request.props["tool"] = json!("other");
        assert!(!selected.asks(Component::ToolUse, &request));
        assert!(!selected.asks(Component::Spinner, &request));
        request.component = Component::Pane;
        assert!(selected.asks(Component::Pane, &request));
        assert!(!Matched::default().asks(Component::Pane, &request));
    }
}
