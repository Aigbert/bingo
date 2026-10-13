//! Validate the settings vocabulary the module host can enforce, failing closed otherwise.
use serde_json::Value;

pub fn validate(schema: &Value, value: &Value) -> Result<(), String> {
    supported(schema, 0)?;
    check(schema, schema, value, "options", 0)
}
fn supported(schema: &Value, depth: usize) -> Result<(), String> {
    if depth > 64 {
        return Err("config schema nesting limit exceeded".into());
    }
    if schema.is_boolean() {
        return Ok(());
    }
    let fields = schema
        .as_object()
        .ok_or("config schema must be an object or boolean")?;
    for (key, value) in fields {
        if !KEYWORDS.contains(&key.as_str()) {
            return Err(format!("unsupported config schema keyword {key}"));
        }
        match key.as_str() {
            "properties" | "$defs" | "definitions" => {
                for schema in value
                    .as_object()
                    .ok_or_else(|| format!("{key} must be an object"))?
                    .values()
                {
                    supported(schema, depth + 1)?;
                }
            }
            "allOf" | "anyOf" | "oneOf" | "prefixItems" => {
                for schema in value
                    .as_array()
                    .ok_or_else(|| format!("{key} must be an array"))?
                {
                    supported(schema, depth + 1)?;
                }
            }
            "additionalProperties" | "items" | "not" => supported(value, depth + 1)?,
            "type" => {
                let kinds = match value {
                    Value::String(kind) => vec![kind.as_str()],
                    Value::Array(kinds) => kinds
                        .iter()
                        .map(|kind| kind.as_str().ok_or("type names must be strings"))
                        .collect::<Result<Vec<_>, _>>()?,
                    _ => return Err("type must be a string or array".into()),
                };
                if kinds.is_empty()
                    || kinds.iter().any(|kind| {
                        ![
                            "null", "boolean", "object", "array", "string", "number", "integer",
                        ]
                        .contains(kind)
                    })
                {
                    return Err("unknown config schema type".into());
                }
            }
            "required" => {
                if !value
                    .as_array()
                    .is_some_and(|values| values.iter().all(Value::is_string))
                {
                    return Err("required must contain property names".into());
                }
            }
            "enum" => {
                if !value.as_array().is_some_and(|values| !values.is_empty()) {
                    return Err("enum must be a nonempty array".into());
                }
            }
            "uniqueItems" => {
                if !value.is_boolean() {
                    return Err("uniqueItems must be boolean".into());
                }
            }
            _ => (),
        }
    }
    Ok(())
}

fn check(
    root: &Value,
    schema: &Value,
    value: &Value,
    path: &str,
    depth: usize,
) -> Result<(), String> {
    if depth > 64 {
        return Err(format!("{path}: config schema nesting limit exceeded"));
    }
    if let Some(allowed) = schema.as_bool() {
        return if allowed {
            Ok(())
        } else {
            Err(format!("{path}: value is not allowed"))
        };
    }
    let fields = schema
        .as_object()
        .ok_or_else(|| format!("{path}: config schema must be an object or boolean"))?;
    for key in fields.keys() {
        if !KEYWORDS.contains(&key.as_str()) {
            return Err(format!("{path}: unsupported config schema keyword {key}"));
        }
    }
    if let Some(reference) = fields.get("$ref") {
        let pointer = reference
            .as_str()
            .and_then(|reference| reference.strip_prefix('#'))
            .ok_or_else(|| format!("{path}: config references must be local"))?;
        let target = root
            .pointer(pointer)
            .ok_or_else(|| format!("{path}: unresolved config reference {pointer}"))?;
        check(root, target, value, path, depth + 1)?;
    }
    if let Some(kind) = fields.get("type") {
        let kinds: Vec<_> = match kind {
            Value::String(kind) => vec![kind.as_str()],
            Value::Array(kinds) => kinds.iter().filter_map(Value::as_str).collect(),
            _ => Vec::new(),
        };
        if !kinds.iter().any(|kind| typed(kind, value)) {
            return Err(format!("{path}: expected {kind}"));
        }
    }
    if fields
        .get("const")
        .is_some_and(|expected| expected != value)
        || fields.get("enum").is_some_and(|choices| {
            !choices
                .as_array()
                .is_some_and(|choices| choices.contains(value))
        })
    {
        return Err(format!("{path}: value is not an allowed choice"));
    }
    alternatives(root, schema, value, path, depth)?;
    object(root, schema, value, path, depth)?;
    array(root, schema, value, path, depth)?;
    bounds(schema, value, path)
}
const KEYWORDS: &[&str] = &[
    "$schema",
    "$id",
    "$comment",
    "$defs",
    "definitions",
    "$ref",
    "title",
    "description",
    "default",
    "examples",
    "deprecated",
    "readOnly",
    "writeOnly",
    "type",
    "const",
    "enum",
    "allOf",
    "anyOf",
    "oneOf",
    "not",
    "properties",
    "required",
    "additionalProperties",
    "items",
    "prefixItems",
    "minItems",
    "maxItems",
    "uniqueItems",
    "minLength",
    "maxLength",
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "minProperties",
    "maxProperties",
];
fn typed(kind: &str, value: &Value) -> bool {
    match kind {
        "null" => value.is_null(),
        "boolean" => value.is_boolean(),
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "number" => value.is_number(),
        "integer" => value.as_f64().is_some_and(|number| number.fract() == 0.0),
        _ => false,
    }
}
fn alternatives(
    root: &Value,
    schema: &Value,
    value: &Value,
    path: &str,
    depth: usize,
) -> Result<(), String> {
    for key in ["allOf", "anyOf", "oneOf"] {
        if let Some(branches) = schema.get(key) {
            let branches = branches
                .as_array()
                .ok_or_else(|| format!("{path}: {key} must be an array"))?;
            let matches = branches
                .iter()
                .filter(|schema| check(root, schema, value, path, depth + 1).is_ok())
                .count();
            let valid = match key {
                "allOf" => matches == branches.len(),
                "anyOf" => matches > 0,
                _ => matches == 1,
            };
            if !valid {
                return Err(format!("{path}: {key} does not match"));
            }
        }
    }
    if let Some(schema) = schema.get("not")
        && check(root, schema, value, path, depth + 1).is_ok()
    {
        return Err(format!("{path}: matches a forbidden config shape"));
    }
    Ok(())
}
fn object(
    root: &Value,
    schema: &Value,
    value: &Value,
    path: &str,
    depth: usize,
) -> Result<(), String> {
    let Some(fields) = value.as_object() else {
        return Ok(());
    };
    if let Some(required) = schema.get("required") {
        let required = required
            .as_array()
            .ok_or_else(|| format!("{path}: required must be an array"))?;
        for key in required {
            let key = key
                .as_str()
                .ok_or_else(|| format!("{path}: required names must be strings"))?;
            if !fields.contains_key(key) {
                return Err(format!("{path}.{key}: required setting is missing"));
            }
        }
    }
    for (key, value) in fields {
        let child = schema
            .get("properties")
            .and_then(|properties| properties.get(key))
            .or_else(|| schema.get("additionalProperties"));
        if let Some(child) = child {
            check(root, child, value, &format!("{path}.{key}"), depth + 1)?;
        }
    }
    Ok(())
}
fn array(
    root: &Value,
    schema: &Value,
    value: &Value,
    path: &str,
    depth: usize,
) -> Result<(), String> {
    let Some(items) = value.as_array() else {
        return Ok(());
    };
    let prefix = schema.get("prefixItems").and_then(Value::as_array);
    for (index, value) in items.iter().enumerate() {
        if schema.get("uniqueItems") == Some(&Value::Bool(true)) && items[..index].contains(value) {
            return Err(format!("{path}[{index}]: duplicate item"));
        }
        if let Some(child) = prefix
            .and_then(|prefix| prefix.get(index))
            .or_else(|| schema.get("items"))
        {
            check(root, child, value, &format!("{path}[{index}]"), depth + 1)?;
        }
    }
    Ok(())
}
fn bounds(schema: &Value, value: &Value, path: &str) -> Result<(), String> {
    let size = match value {
        Value::String(text) => Some((text.chars().count(), "minLength", "maxLength")),
        Value::Array(items) => Some((items.len(), "minItems", "maxItems")),
        Value::Object(fields) => Some((fields.len(), "minProperties", "maxProperties")),
        _ => None,
    };
    if let Some((size, min, max)) = size {
        for (key, lower) in [(min, true), (max, false)] {
            if let Some(bound) = schema.get(key) {
                let bound = bound
                    .as_u64()
                    .ok_or_else(|| format!("{path}: {key} must be a nonnegative integer"))?;
                if (lower && (size as u64) < bound) || (!lower && (size as u64) > bound) {
                    return Err(format!("{path}: violates {key}"));
                }
            }
        }
    }
    if let Some(number) = value.as_f64() {
        for (key, lower, exclusive) in [
            ("minimum", true, false),
            ("maximum", false, false),
            ("exclusiveMinimum", true, true),
            ("exclusiveMaximum", false, true),
        ] {
            if let Some(bound) = schema.get(key) {
                let bound = bound
                    .as_f64()
                    .ok_or_else(|| format!("{path}: {key} must be a number"))?;
                if (lower && number < bound)
                    || (!lower && number > bound)
                    || (exclusive && number == bound)
                {
                    return Err(format!("{path}: violates {key}"));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn unsupported_validation_cannot_hide_inside_not() {
        assert!(
            validate(
                &serde_json::json!({"not":{"pattern":"x"}}),
                &serde_json::json!("y")
            )
            .unwrap_err()
            .contains("unsupported")
        );
    }
    #[test]
    fn validates_nested_options_references_and_choices() {
        let schema = json!({"$defs":{"label":{"type":"string","minLength":1}},"type":"object","properties":{"label":{"$ref":"#/$defs/label"}},"required":["label"],"additionalProperties":false});
        assert!(validate(&schema, &json!({"label":"ok"})).is_ok());
        for value in [
            json!({}),
            json!({"label":1}),
            json!({"label":""}),
            json!({"label":"ok","unknown":1}),
        ] {
            assert!(validate(&schema, &value).is_err());
        }
        assert!(validate(&json!({"oneOf":[{"const":1},{"type":"string"}]}), &json!(1)).is_ok());
        assert!(validate(&json!({"not":{"const":1}}), &json!(1)).is_err());
        assert!(
            validate(
                &json!({"type":"array","items":{"type":"integer"},"uniqueItems":true}),
                &json!([1, 1])
            )
            .is_err()
        );
        assert!(
            validate(&json!({"pattern":"x"}), &json!("x"))
                .unwrap_err()
                .contains("unsupported")
        );
    }
}
