use bingo_sdk::Catalog;
use serde_json::{Value, json};

pub(super) fn events(catalog: &Catalog) -> String {
    let mut result = String::from("// Generated from the live host event catalogue.\n");
    for entry in &catalog.entries {
        result.push_str(&event_namespaces(entry));
    }
    for field in ["input", "result"] {
        result.push_str(&format!("export interface Event{}s {{\n", title(field)));
        for entry in &catalog.entries {
            if let Some(text) = event_doc(&entry.id, &entry.meta) {
                result.push_str(&bingo_schema_ts::doc(&text, ""));
            }
            result.push_str(&format!(
                "{}: {}{}.Payload;\n",
                json!(entry.id),
                title(&entry.id),
                title(field)
            ));
        }
        result.push_str("}\n");
    }
    result
}

fn event_namespaces(entry: &bingo_sdk::CatalogEntry) -> String {
    let title = title(&entry.id);
    let mut result = String::new();
    for field in ["input", "result"] {
        let schema = payload(&entry.meta[field]);
        result.push_str(&format!(
            "export namespace {title}{} {{\n{}\n}}\n",
            self::title(field),
            bingo_schema_ts::render(&schema)
        ));
    }
    if let Some(text) = event_doc(&entry.id, &entry.meta) {
        result.push_str(&bingo_schema_ts::doc(&text, ""));
    }
    result.push_str(&format!("export type {title} = {title}Input.Payload;\n"));
    result
}

/// What an event is and what `next` does, with the fields a matcher may
/// name.
fn event_doc(id: &str, meta: &Value) -> Option<String> {
    let text = event_text(id, meta)?;
    let fields: Vec<String> = meta["matcher"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|field| format!("`{field}`"))
        .collect();
    Some(if fields.is_empty() {
        text
    } else {
        format!("{text}\n\nMatch on: {}.", fields.join(", "))
    })
}

/// A kernel event carries its catalogue doc; a service face falls back to
/// the `$` row that calls it.
pub(super) fn event_text(id: &str, meta: &Value) -> Option<String> {
    let described = meta["description"].as_str().map(str::to_string);
    described.or_else(|| {
        let (noun, method) = id.split_once('.')?;
        let row = crate::dollar::table::ROWS
            .iter()
            .find(|row| row.noun == noun && row.method == method)?;
        Some(row.doc.trim().to_string())
    })
}

pub(super) fn tools(catalog: &Catalog) -> String {
    let mut result = String::from("// Generated from this host's tool input schemas.\n");
    for (index, entry) in catalog.entries.iter().enumerate() {
        let schema = payload(&entry.meta["inputSchema"]);
        result.push_str(&format!(
            "export namespace Tool{index} {{\n{}\n}}\n",
            bingo_schema_ts::render(&schema)
        ));
    }
    result.push_str("export interface ToolInputs {\n");
    for (index, entry) in catalog.entries.iter().enumerate() {
        result.push_str(&format!("{}: Tool{index}.Payload;\n", json!(entry.id)));
    }
    result.push_str("}\n");
    result
}
pub(super) fn title(name: &str) -> String {
    name.split('.')
        .map(|part| {
            let mut chars = part.chars();
            let mut title = chars
                .next()
                .map(|ch| ch.to_uppercase().to_string())
                .unwrap_or_default();
            title.extend(chars);
            title
                .chars()
                .map(|ch| {
                    if ch.is_ascii_alphanumeric() {
                        ch.to_string()
                    } else {
                        format!("_{:x}_", u32::from(ch))
                    }
                })
                .collect::<String>()
        })
        .collect()
}
pub(super) fn payload(schema: &Value) -> Value {
    let mut schema = if schema.is_object() {
        schema.clone()
    } else {
        json!({"allOf":[schema]})
    };
    if !["type", "$ref", "oneOf", "anyOf", "allOf", "enum", "const"]
        .iter()
        .any(|key| schema.get(key).is_some())
    {
        schema["allOf"] = json!([true]);
    }
    schema["title"] = json!("Payload");
    schema
}

pub(super) fn services(catalog: &Catalog) -> String {
    let mut nouns = std::collections::BTreeMap::<&str, Vec<(&str, &str)>>::new();
    for entry in &catalog.entries {
        let Some((noun, method)) = entry.id.split_once('.') else {
            continue;
        };
        let owned = crate::dollar::table::ROWS.iter().any(|row| {
            row.noun == noun && !matches!(row.door, crate::dollar::table::Door::Service(_))
        });
        if entry.meta.get("unsafe").is_some() && !owned && noun != "ui" {
            nouns.entry(noun).or_default().push((method, &entry.id));
        }
    }
    let mut result = String::from(
        "import type {EventInputs,EventResults} from './events.ts';\ntype AtSite<T> = T extends object ? Omit<T,'session'|'turn'> & Partial<Pick<T,Extract<keyof T,'session'|'turn'>>> : T;\nexport interface Services {\n",
    );
    for (noun, methods) in nouns {
        result.push_str(&format!("{}: {{\n", json!(noun)));
        for (method, event) in methods {
            result.push_str(&format!(
                "{}(input: AtSite<EventInputs[{}]>): Promise<EventResults[{}]>;\n",
                json!(method),
                json!(event),
                json!(event)
            ));
        }
        result.push_str("};\n");
    }
    result.push_str("}\n");
    result
}
