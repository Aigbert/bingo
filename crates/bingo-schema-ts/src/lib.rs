//! Erasable TypeScript declarations from JSON Schema.
use serde_json::Value;

/// Render a schema's definitions and its titled root, if any.
pub fn render(schema: &Value) -> String {
    let mut output = String::from("// Generated from JSON Schema. Do not edit.\n");
    if let Some(definitions) = schema.get("$defs").and_then(Value::as_object) {
        let sorted: std::collections::BTreeMap<_, _> = definitions.iter().collect();
        for (name, definition) in sorted {
            output.push_str(&format!(
                "export type {} = {};\n",
                identifier(name),
                expression(definition)
            ));
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
        output.push_str(&format!(
            "export type {} = {};\n",
            identifier(title),
            expression(schema)
        ));
    }
    output
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

fn joined(values: &[Value], separator: &str) -> String {
    format!(
        "({})",
        values
            .iter()
            .map(expression)
            .collect::<Vec<_>>()
            .join(separator)
    )
}

fn expression(schema: &Value) -> String {
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
            return joined(values, separator);
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
        return joined(&alternatives, " | ");
    }
    match schema.get("type").and_then(Value::as_str) {
        Some("string") => "string".into(),
        Some("integer" | "number") => "number".into(),
        Some("boolean") => "boolean".into(),
        Some("null") => "null".into(),
        Some("array") => array(schema),
        Some("object") => object(schema),
        _ if schema.get("properties").is_some() => object(schema),
        _ => "unknown".into(),
    }
}

fn array(schema: &Value) -> String {
    if let Some(prefix) = schema.get("prefixItems").and_then(Value::as_array) {
        let mut fields: Vec<_> = prefix.iter().map(expression).collect();
        if schema.get("items") != Some(&Value::Bool(false)) {
            fields.push(format!("...Array<{}>", expression(&schema["items"])));
        }
        return format!("[{}]", fields.join(", "));
    }
    format!("Array<{}>", expression(&schema["items"]))
}

fn object(schema: &Value) -> String {
    let mut fields = Vec::new();
    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        let sorted: std::collections::BTreeMap<_, _> = properties.iter().collect();
        for (name, value) in sorted {
            let required = schema
                .get("required")
                .and_then(Value::as_array)
                .is_some_and(|fields| fields.iter().any(|field| field.as_str() == Some(name)));
            let optional = if required { "" } else { "?" };
            fields.push(format!(
                "{}{optional}: {};",
                Value::String(name.clone()),
                expression(value)
            ));
        }
    }
    if let Some(additional) = schema.get("additionalProperties") {
        if additional != &Value::Bool(false) {
            fields.push(format!("[key: string]: {};", expression(additional)));
        }
    } else {
        fields.push("[key: string]: unknown;".into());
    }
    format!("{{ {} }}", fields.join(" "))
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
    fn definitions_are_sorted_and_references_stay_references() {
        let schema = json!({"$defs":{"Z":{"$ref":"#/$defs/A"},"A":{"const":"ok"}}});
        let rendered = render(&schema);
        assert!(rendered.contains("export type A = \"ok\";\n"));
        assert!(rendered.contains("export type Z = A;\n"));
        assert!(rendered.find("type A") < rendered.find("type Z"));
    }
}
