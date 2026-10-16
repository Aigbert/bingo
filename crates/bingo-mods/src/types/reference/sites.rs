//! The drawing sections: the elements a view is built from, and the sites
//! (`Component`s) a `ui.render` hook can match.
use super::{fields, one_line};
use bingo_sdk::render::{Component, SurfaceName};
use bingo_sdk::view::table;
use serde_json::{Value, json};

const ELEMENTS: &str = "## Elements\n\nA view is `{kind, ...props}`. Get each `kind` from `const {Box, Text} = await $.ui.resolve(event)`: a surface that cannot draw an element gets its plain-text fold. Handlers (`onPress`, `onKey`, `onInput`, `onSubmit`, `onSelect`) are functions taking `($, value)`; they may write state.\n\n";

const SITES: &str = "## Sites\n\nMatch `ui.render` with `{component, requestId?, props?}`. Return a view to draw the site, or wrap `await next(event)`, which is bingo's own drawing. Sealed sites must keep that drawing exactly once and unchanged.\n\n";

pub(super) fn elements() -> String {
    let mut text = String::from(ELEMENTS);
    for element in crate::types::elements::described() {
        text.push_str(&format!("### `{}`\n\n", element.kind));
        if let Some(doc) = &element.doc {
            text.push_str(&format!("{}\n\n", one_line(doc)));
        }
        text.push_str(&format!("Drawn on: {}.\n\n", drawn_on(element.kind)));
        for (prop, doc) in &element.props {
            let doc = doc.as_deref().map(one_line).unwrap_or_default();
            text.push_str(&format!("- `{prop}` — {doc}\n"));
        }
        text.push('\n');
    }
    text
}

fn drawn_on(kind: &str) -> String {
    let surfaces: Vec<String> = [table::TERMINAL, table::DESKTOP, table::CHAT]
        .into_iter()
        .filter(|surface| {
            table::elements(*surface)
                .iter()
                .any(|element| element.kind == kind)
        })
        .map(name)
        .collect();
    if surfaces.is_empty() {
        "no surface natively".into()
    } else {
        surfaces.join(", ")
    }
}

fn name(surface: SurfaceName) -> String {
    format!("`{}`", json!(surface).as_str().unwrap_or_default())
}

pub(super) fn components() -> String {
    let schema = serde_json::to_value(schemars::schema_for!(bingo_sdk::component::Props))
        .unwrap_or(Value::Null);
    let mut text = String::from(SITES);
    for component in Component::ALL {
        text.push_str(&site(&schema, component));
    }
    text
}

fn site(schema: &Value, component: Component) -> String {
    let id = json!(component);
    let id = id.as_str().unwrap_or_default();
    let surfaces: Vec<String> = component.surfaces().iter().copied().map(name).collect();
    let sealed = if component.sealed() { " Sealed." } else { "" };
    let mut text = format!(
        "### `{id}`\n\n{} Asked by: {}.{sealed}\n\nProps:\n",
        component.summary(),
        surfaces.join(", ")
    );
    let props = schema["oneOf"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|variant| variant["properties"]["component"]["const"] == id)
        .map(|variant| &variant["properties"]["props"]);
    text.push_str(&props.map(|props| fields(schema, props)).unwrap_or_default());
    text.push('\n');
    text
}
