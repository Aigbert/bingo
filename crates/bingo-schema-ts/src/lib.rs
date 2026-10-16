//! Erasable TypeScript declarations from JSON Schema.
use serde_json::Value;

/// Render a schema's definitions and its titled root, if any. A schema's
/// `description` travels as a JSDoc comment on its declaration and on each
/// object property, so a reader of the types sees what the schema says.
pub fn render(schema: &Value) -> String {
    let mut output = String::from("// Generated from JSON Schema. Do not edit.\n");
    if let Some(definitions) = schema.get("$defs").and_then(Value::as_object) {
        let sorted: std::collections::BTreeMap<_, _> = definitions.iter().collect();
        for (name, definition) in sorted {
            output.push_str(&declaration(name, definition));
        }
    }
    if let Some(title) = schema.get("title").and_then(Value::as_str)
        && schema
            .get("$defs")
            .and_then(|defs| defs.get(title))
            .is_none()
        && ["type", "$ref", "oneOf", "anyOf", "allOf", "enum", "const"]
            .iter()
            .any(|key| schema.get(key).is_some())
    {
        output.push_str(&declaration(title, schema));
    }
    output
}

fn declaration(name: &str, schema: &Value) -> String {
    format!(
        "{}export type {} = {};\n",
        described(schema, ""),
        identifier(name),
        expression(schema, 0)
    )
}

/// A JSDoc block for `text`, each line prefixed by `indent`; `*/` inside the
/// text is broken so it cannot end the comment early.
pub fn doc(text: &str, indent: &str) -> String {
    let text = text.trim().replace("*/", "*\\/");
    let lines: Vec<&str> = text.lines().collect();
    match lines.as_slice() {
        [] => String::new(),
        [line] => format!("{indent}/** {line} */\n"),
        lines => {
            let mut block = format!("{indent}/**\n");
            for line in lines {
                let line = line.trim_end();
                if line.is_empty() {
                    block.push_str(&format!("{indent} *\n"));
                } else {
                    block.push_str(&format!("{indent} * {line}\n"));
                }
            }
            block.push_str(&format!("{indent} */\n"));
            block
        }
    }
}

fn described(schema: &Value, indent: &str) -> String {
    schema
        .get("description")
        .and_then(Value::as_str)
        .map(|text| doc(text, indent))
        .unwrap_or_default()
}

fn identifier(name: &str) -> String {
    // Encoding punctuation avoids collisions between names like `A-B` and `A_B`.
    let mut result = String::from("_");
    for (index, ch) in name.chars().enumerate() {
        if ch.is_ascii_alphabetic() || (index > 0 && ch.is_ascii_digit()) {
            result.push(ch);
        } else {
            result.push_str(&format!("_{:x}_", u32::from(ch)));
        }
    }
    if name
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic())
        && name.chars().all(|ch| ch.is_ascii_alphanumeric())
    {
        name.into()
    } else {
        result
    }
}

fn joined(values: &[Value], separator: &str, depth: usize) -> String {
    format!(
        "({})",
        values
            .iter()
            .map(|value| expression(value, depth))
            .collect::<Vec<_>>()
            .join(separator)
    )
}

/// The TypeScript type of `schema`, nested `depth` objects deep.
pub fn expression(schema: &Value, depth: usize) -> String {
    if schema == &Value::Bool(false) {
        return "never".into();
    }
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        return reference
            .strip_prefix("#/$defs/")
            .map(|name| identifier(&name.replace("~1", "/").replace("~0", "~")))
            .unwrap_or_else(|| "unknown".into());
    }
    if let Some(value) = schema.get("const") {
        return value.to_string();
    }
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        return format!(
            "({})",
            values
                .iter()
                .map(Value::to_string)
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
    for (key, separator) in [("oneOf", " | "), ("anyOf", " | "), ("allOf", " & ")] {
        if let Some(values) = schema.get(key).and_then(Value::as_array) {
            return joined(values, separator, depth);
        }
    }
    if let Some(types) = schema.get("type").and_then(Value::as_array) {
        let alternatives: Vec<_> = types
            .iter()
            .map(|kind| {
                let mut alternative = schema.clone();
                alternative["type"] = kind.clone();
                alternative
            })
            .collect();
        return joined(&alternatives, " | ", depth);
    }
    match schema.get("type").and_then(Value::as_str) {
        Some("string") => "string".into(),
        Some("integer" | "number") => "number".into(),
        Some("boolean") => "boolean".into(),
        Some("null") => "null".into(),
        Some("array") => array(schema, depth),
        Some("object") => object(schema, depth),
        _ if schema.get("properties").is_some() => object(schema, depth),
        _ => "unknown".into(),
    }
}

fn array(schema: &Value, depth: usize) -> String {
    if let Some(prefix) = schema.get("prefixItems").and_then(Value::as_array) {
        let mut fields: Vec<_> = prefix.iter().map(|item| expression(item, depth)).collect();
        if schema.get("items") != Some(&Value::Bool(false)) {
            fields.push(format!("...Array<{}>", expression(&schema["items"], depth)));
        }
        return format!("[{}]", fields.join(", "));
    }
    format!("Array<{}>", expression(&schema["items"], depth))
}

/// An object stays on one line until one of its properties is described;
/// then each property takes a line under its comment.
fn object(schema: &Value, depth: usize) -> String {
    let properties: std::collections::BTreeMap<_, _> = schema
        .get("properties")
        .and_then(Value::as_object)
        .map(|properties| properties.iter().collect())
        .unwrap_or_default();
    let documented = properties
        .values()
        .any(|value| value.get("description").is_some());
    let indent = "  ".repeat(depth + 1);
    let mut fields = Vec::new();
    for (name, value) in properties {
        let optional = if required(schema, name) { "" } else { "?" };
        let declaration = format!(
            "{}{optional}: {};",
            Value::String(name.clone()),
            expression(value, depth + 1)
        );
        fields.push((described(value, &indent), declaration));
    }
    if let Some(additional) = schema.get("additionalProperties") {
        if additional != &Value::Bool(false) {
            let declaration = format!("[key: string]: {};", expression(additional, depth + 1));
            fields.push((String::new(), declaration));
        }
    } else {
        fields.push((String::new(), "[key: string]: unknown;".into()));
    }
    if !documented {
        let fields: Vec<_> = fields.into_iter().map(|(_, field)| field).collect();
        return format!("{{ {} }}", fields.join(" "));
    }
    let lines: Vec<String> = fields
        .into_iter()
        .map(|(comment, declaration)| format!("{comment}{indent}{declaration}"))
        .collect();
    format!("{{\n{}\n{}}}", lines.join("\n"), "  ".repeat(depth))
}

fn required(schema: &Value, name: &str) -> bool {
    schema
        .get("required")
        .and_then(Value::as_array)
        .is_some_and(|fields| fields.iter().any(|field| field.as_str() == Some(name)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn small_schemas_render_without_runtime_types() {
        for (schema, expected) in [
            (
                json!({"type":"object","properties":{"name":{"type":"string"},"age":{"type":"integer"}},"required":["name"],"additionalProperties":false}),
                "{ \"age\"?: number; \"name\": string; }",
            ),
            (
                json!({"oneOf":[{"type":"string"},{"type":"null"}]}),
                "(string | null)",
            ),
            (json!({"enum":["a","b"]}), "(\"a\" | \"b\")"),
            (
                json!({"type":"array","items":{"$ref":"#/$defs/Thing"}}),
                "Array<Thing>",
            ),
            (json!({"type":["string","null"]}), "(string | null)"),
            (
                json!({"type":"object","additionalProperties":{"type":"number"}}),
                "{ [key: string]: number; }",
            ),
            (json!(false), "never"),
            (json!(true), "unknown"),
        ] {
            let mut root = json!({"title":"Example","$defs":{"Example":schema}});
            assert!(
                render(&root).contains(&format!("export type Example = {expected};")),
                "{root}"
            );
            root["title"] = json!("ignored");
        }
    }

    #[test]
    fn descriptions_become_jsdoc_on_declarations_and_properties() {
        let schema = json!({"$defs":{"Thing":{"description":"A thing.\nTwo lines.",
            "type":"object","additionalProperties":false,"required":["name"],
            "properties":{"name":{"type":"string","description":"Its name; never */ early."},
                "size":{"type":"integer"}}}}});
        let rendered = render(&schema);
        assert!(
            rendered.contains("/**\n * A thing.\n * Two lines.\n */\nexport type Thing = {\n"),
            "{rendered}"
        );
        assert!(
            rendered.contains("  /** Its name; never *\\/ early. */\n  \"name\": string;\n  \"size\"?: number;\n};"),
            "{rendered}"
        );
    }

    #[test]
    fn definitions_are_sorted_and_references_stay_references() {
        let schema = json!({"$defs":{"Z":{"$ref":"#/$defs/A"},"A":{"const":"ok"}}});
        let rendered = render(&schema);
        assert!(rendered.contains("export type A = \"ok\";\n"));
        assert!(rendered.contains("export type Z = A;\n"));
        assert!(rendered.find("type A") < rendered.find("type Z"));
    }
}
