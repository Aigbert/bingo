//! Checked views to Card 2.0. Text never becomes callback structure.
mod controls;
mod markdown;
#[cfg(test)]
mod tests;
use bingo_channel_adapter::{
    conversation::Conversation,
    drawing::{Drawing, engine},
};
use bingo_sdk::{View, view::Direction};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const ELEMENTS: usize = 200;
const BYTES: usize = 30_000;

pub fn card(drawing: &Drawing, to: &Conversation, images: &BTreeMap<String, String>) -> Value {
    let context = Context {
        drawing,
        to,
        images,
    };
    let mut elements = context.elements(&drawing.view);
    if !drawing.footer.is_empty() {
        elements.push(markdown::element(&format!(
            "---\n{}",
            markdown::escape(&drawing.footer.chars().take(2048).collect::<String>())
        )));
    }
    let card = envelope(elements);
    if count(&card) <= ELEMENTS && card.to_string().len() <= BYTES {
        card
    } else {
        fallback(drawing)
    }
}

fn envelope(elements: Vec<Value>) -> Value {
    json!({"schema":"2.0","config":{"streaming_mode":false,"update_multi":true},"body":{"elements":elements}})
}

fn count(value: &Value) -> usize {
    match value {
        Value::Object(fields) => {
            usize::from(fields.contains_key("tag")) + fields.values().map(count).sum::<usize>()
        }
        Value::Array(values) => values.iter().map(count).sum(),
        _ => 0,
    }
}

fn fallback(drawing: &Drawing) -> Value {
    let mut text = fold(&drawing.view, &drawing.request);
    let footer = markdown::element(&format!(
        "---\n{}",
        markdown::escape(&drawing.footer.chars().take(2048).collect::<String>())
    ));
    loop {
        let card = envelope(vec![markdown::element(&text), footer.clone()]);
        if card.to_string().len() <= BYTES {
            return card;
        }
        let keep = text.chars().count() / 2;
        text = text.chars().take(keep).collect::<String>();
        text.push('…');
    }
}

fn fold(view: &View, request: &bingo_sdk::render::RenderRequest) -> String {
    match view {
        View::Engine { component, props } => engine(*component, props, request),
        View::Box { children, .. } => children
            .iter()
            .map(|child| fold(child, request))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => markdown::escape(&view.fold_markdown()),
    }
}

struct Context<'a> {
    drawing: &'a Drawing,
    to: &'a Conversation,
    images: &'a BTreeMap<String, String>,
}
impl Context<'_> {
    fn elements(&self, view: &View) -> Vec<Value> {
        match view {
            View::Box { children, layout } => {
                if layout.display.as_deref() == Some("none") {
                    return vec![];
                }
                if layout.flex_direction == Some(Direction::Row) {
                    vec![
                        json!({"tag":"column_set","flex_mode":"flow","columns":children.iter().map(|child|
                        json!({"tag":"column","width":"weighted","weight":1,"elements":self.elements(child)})).collect::<Vec<_>>()}),
                    ]
                } else {
                    children.iter().flat_map(|v| self.elements(v)).collect()
                }
            }
            View::Button { button, children } => vec![self.button(button, children)],
            View::Select { select } => vec![self.select(select)],
            View::Input { input } => vec![self.input(input)],
            View::Image { source, alt, .. } => vec![match self.images.get(source) {
                Some(key) => {
                    json!({"tag":"img","img_key":key,"alt":{"tag":"plain_text","content":alt}})
                }
                None => markdown::element(&markdown::escape(alt)),
            }],
            View::Engine { component, props } => vec![markdown::element(&engine(
                *component,
                props,
                &self.drawing.request,
            ))],
            _ => vec![markdown::element(&markdown::view(view))],
        }
    }
}

pub fn image_sources(view: &View) -> Vec<String> {
    match view {
        View::Image { source, .. } => vec![source.clone()],
        View::Box { children, .. } => children.iter().flat_map(image_sources).collect(),
        _ => vec![],
    }
}
