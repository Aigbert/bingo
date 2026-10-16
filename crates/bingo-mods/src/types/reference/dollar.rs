//! The `$` section: each noun with its example, then each method with its
//! input fields. A row's call takes one input object; `session` and `turn`
//! default to the hook's.
use super::{one_line, resolve};
use crate::dollar::nouns::{CALLS, NOUNS, Noun, calls};
use crate::dollar::table::{ROWS, Row};
use serde_json::Value;

const HEAD: &str = "## The `$` nouns\n\nEvery method is asynchronous. A method taking one input object fills `session` and `turn` from the hook when they are left out. A noun must be in `uses` in `plugin.json`; methods marked unsafe also need authority a project mod is not given by default.\n\n";

pub(super) fn section() -> String {
    let schema = crate::dollar::table::document();
    let mut text = String::from(HEAD);
    for noun in NOUNS {
        text.push_str(&noun_section(&schema, noun));
    }
    text
}

fn noun_section(schema: &Value, noun: &Noun) -> String {
    let mut text = format!(
        "### `$.{}`\n\n{}\n\n```ts\n{}\n```\n\n",
        noun.name, noun.doc, noun.example
    );
    for call in calls(noun.name) {
        let doc = call
            .doc
            .or_else(|| row(noun.name, call.method).map(|row| row.doc.trim()))
            .unwrap_or_default();
        text.push_str(&format!(
            "- `$.{}.{}` — {}\n",
            noun.name,
            call.signature,
            one_line(doc)
        ));
    }
    for row in ROWS.iter().filter(|row| row.noun == noun.name) {
        if !CALLS
            .iter()
            .any(|call| call.noun == row.noun && call.method == row.method)
        {
            text.push_str(&method(schema, row));
        }
    }
    text.push('\n');
    text
}

fn row(noun: &str, method: &str) -> Option<&'static Row> {
    ROWS.iter()
        .find(|row| row.noun == noun && row.method == method)
}

fn method(schema: &Value, row: &Row) -> String {
    let name = format!("{}.{}", row.noun, row.method);
    let entry = &schema["methods"][&name];
    let result = bingo_schema_ts::expression(&entry["result"], 0);
    let caution = if row.unsafe_call { " Unsafe." } else { "" };
    let mut text = format!(
        "- `$.{name}({})` → `{result}` — {}{caution}\n",
        input(schema, &entry["input"]),
        one_line(row.doc)
    );
    for (field, doc) in described(schema, &entry["input"]) {
        text.push_str(&format!("  - `{field}`: {}\n", one_line(doc)));
    }
    text
}

/// The call's argument as an author writes it: nothing when the hook's site
/// fills it, `{field, optional?}` for an object, or the type.
fn input(schema: &Value, input: &Value) -> String {
    if *input == Value::Bool(true) {
        return "input".into();
    }
    let resolved = resolve(schema, input);
    if resolved["type"] == "null" {
        return String::new();
    }
    let Some(properties) = resolved["properties"].as_object() else {
        return bingo_schema_ts::expression(input, 0);
    };
    let required = |name: &str| {
        resolved["required"]
            .as_array()
            .is_some_and(|names| names.iter().any(|value| value == name))
    };
    let names: Vec<String> = properties
        .keys()
        .filter(|name| !matches!(name.as_str(), "session" | "turn"))
        .map(|name| format!("{name}{}", if required(name) { "" } else { "?" }))
        .collect();
    if names.is_empty() {
        return String::new();
    }
    format!("{{{}}}", names.join(", "))
}

fn described<'a>(schema: &'a Value, input: &'a Value) -> Vec<(&'a str, &'a str)> {
    resolve(schema, input)["properties"]
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(name, field)| Some((name.as_str(), field["description"].as_str()?)))
        .filter(|(name, _)| !matches!(*name, "session" | "turn"))
        .collect()
}
