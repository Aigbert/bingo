use super::*;
use bingo_sdk::{
    Action,
    view::{ButtonProps, InputProps, SelectProps, TextChild, text_children},
};

impl Context<'_> {
    fn callback(&self, action: &Action) -> Value {
        json!({"type":"callback","value":super::super::event::action_value(
            self.to, &self.drawing.session, &self.drawing.request.request_id, action)})
    }

    pub(super) fn button(&self, props: &ButtonProps, children: &[TextChild]) -> Value {
        let label = props
            .label
            .clone()
            .unwrap_or_else(|| text_children(children));
        let mut button = json!({"tag":"button","text":{"tag":"plain_text","content":label},
            "type": if props.variant.as_deref() == Some("primary") { "primary" } else { "default" }});
        match &props.on_press {
            Some(action) => button["behaviors"] = json!([self.callback(action)]),
            None => button["disabled"] = json!(true),
        }
        button
    }

    pub(super) fn select(&self, props: &SelectProps) -> Value {
        let options: Vec<_> = props.options.iter().flatten().map(|option|
            json!({"text":{"tag":"plain_text","content":option.label},"value":option.value})).collect();
        let mut select = json!({"tag":"select_static","placeholder":{"tag":"plain_text","content":props.label.as_deref().unwrap_or("Select")},"options":options});
        if let Some(value) = &props.value {
            select["initial_option"] = json!(value);
        }
        match &props.on_select {
            Some(action) => select["behaviors"] = json!([self.callback(action)]),
            None => select["disabled"] = json!(true),
        }
        select
    }

    pub(super) fn input(&self, props: &InputProps) -> Value {
        let mut input = json!({"tag":"input","name":"value",
            "placeholder":{"tag":"plain_text","content":props.placeholder.as_deref().unwrap_or("")},
            "default_value":props.value.as_deref().unwrap_or("")});
        if let Some(label) = &props.label {
            input["label"] = json!({"tag":"plain_text","content":label});
        }
        let Some(action) = &props.on_submit else {
            return markdown::element(&markdown::escape(props.value.as_deref().unwrap_or("")));
        };
        json!({"tag":"form","name":props.key.as_deref().unwrap_or("input"),"elements":[input,
            {"tag":"button","name":"submit","action_type":"form_submit","type":"primary",
             "text":{"tag":"plain_text","content":"Submit"},"behaviors":[self.callback(action)]}]})
    }
}
