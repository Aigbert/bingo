//! The elements a mod draws with, described from the `View` schema: the
//! element table names the props, the Rust doc comments say what each does.
use bingo_sdk::view::table::ELEMENTS;
use serde_json::{Value, json};

/// One element as an author meets it.
pub(super) struct Described {
    pub kind: &'static str,
    pub doc: Option<String>,
    pub props: Vec<(&'static str, Option<String>)>,
}

pub(super) fn described() -> Vec<Described> {
    let schema =
        serde_json::to_value(schemars::schema_for!(bingo_sdk::View)).unwrap_or(Value::Null);
    ELEMENTS
        .iter()
        .map(|element| {
            let variant = variant(&schema, element.kind);
            let mut fields = serde_json::Map::new();
            properties(&schema, variant, &mut fields);
            Described {
                kind: element.kind,
                doc: text(variant),
                props: element
                    .props
                    .iter()
                    .map(|prop| (*prop, fields.get(*prop).and_then(text)))
                    .collect(),
            }
        })
        .collect()
}

/// The JSX declarations: the `View` types, `h`, and one intrinsic element
/// per table row with its props, each carrying its doc.
pub(super) fn declarations() -> String {
    let schema =
        serde_json::to_value(schemars::schema_for!(bingo_sdk::View)).unwrap_or(Value::Null);
    let mut result = bingo_schema_ts::render(&schema);
    result.push_str("export type Element = View;\nexport declare function h(kind: unknown, props: unknown, ...children: unknown[]): Element;\nexport declare const Fragment: 'Fragment';\ndeclare global { namespace JSX { type Element = import('./elements').Element; interface IntrinsicElements {\n");
    for element in described() {
        result.push_str(&intrinsic(&element));
    }
    result.push_str("} } function h(kind: unknown, props: unknown, ...children: unknown[]): JSX.Element; const Fragment: 'Fragment'; }\n");
    result
}

fn intrinsic(element: &Described) -> String {
    let mut result = element
        .doc
        .as_deref()
        .map(|doc| bingo_schema_ts::doc(doc, ""))
        .unwrap_or_default();
    result.push_str(&format!("{}: {{\n", json!(element.kind)));
    for (prop, doc) in &element.props {
        if let Some(doc) = doc {
            result.push_str(&bingo_schema_ts::doc(doc, "  "));
        }
        result.push_str(&format!("  {}?: any;\n", json!(prop)));
    }
    result.push_str("};\n");
    result
}

fn variant<'a>(schema: &'a Value, kind: &str) -> &'a Value {
    schema["oneOf"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|variant| variant["properties"]["kind"]["const"] == kind)
        .unwrap_or(&Value::Null)
}

/// Every property a variant has, through `$ref` and `allOf`: a flattened
/// style struct lands as either.
fn properties(root: &Value, schema: &Value, into: &mut serde_json::Map<String, Value>) {
    if let Some(reference) = schema["$ref"].as_str()
        && let Some(target) = root.pointer(reference.trim_start_matches('#'))
    {
        properties(root, target, into);
    }
    for part in schema["allOf"].as_array().into_iter().flatten() {
        properties(root, part, into);
    }
    for (name, field) in schema["properties"].as_object().into_iter().flatten() {
        into.entry(name.clone()).or_insert_with(|| field.clone());
    }
}

fn text(schema: &Value) -> Option<String> {
    schema["description"].as_str().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_element_and_prop_carries_its_doc_into_the_jsx_types() {
        let declared = declarations();
        for element in described() {
            let doc = element
                .doc
                .unwrap_or_else(|| panic!("{} has no doc", element.kind));
            assert!(
                declared.contains(&bingo_schema_ts::doc(&doc, "")),
                "{}",
                element.kind
            );
            for (prop, doc) in element.props {
                let doc = doc.unwrap_or_else(|| panic!("{}.{prop} has no doc", element.kind));
                assert!(declared.contains(&format!(
                    "{}  {}?: any;",
                    bingo_schema_ts::doc(&doc, "  "),
                    json!(prop)
                )));
            }
        }
    }
}
