//! Erasable TypeScript declarations from JSON Schema.
use serde_json::Value;

/// Render a schema's definitions and its titled root, if any.
pub fn render(_schema: &Value) -> String {
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn small_schemas_render_without_runtime_types() {
        for (schema, expected) in [
            (json!({"type":"object","properties":{"name":{"type":"string"},"age":{"type":"integer"}},"required":["name"],"additionalProperties":false}), "{ \"age\"?: number; \"name\": string; }"),
            (json!({"oneOf":[{"type":"string"},{"type":"null"}]}), "(string | null)"),
            (json!({"enum":["a","b"]}), "(\"a\" | \"b\")"),
            (json!({"type":"array","items":{"$ref":"#/$defs/Thing"}}), "Array<Thing>"),
            (json!({"type":["string","null"]}), "(string | null)"),
            (json!({"type":"object","additionalProperties":{"type":"number"}}), "{ [key: string]: number; }"),
            (json!(false), "never"),
            (json!(true), "unknown"),
        ] {
            let mut root = json!({"title":"Example","$defs":{"Example":schema}});
            assert!(render(&root).contains(&format!("export type Example = {expected};")), "{root}");
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
