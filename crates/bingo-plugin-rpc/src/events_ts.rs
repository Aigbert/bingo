//! Event types are rendered by the same library as the protocol types.
#[cfg(test)]
mod tests {
    use bingo_sdk::catalogue;
    use serde_json::Value;

    fn title(name: &str) -> String {
        name.split('.')
            .map(|part| {
                let mut chars = part.chars();
                chars
                    .next()
                    .map(|head| head.to_uppercase().to_string() + chars.as_str())
                    .unwrap_or_default()
            })
            .collect()
    }

    fn rendered() -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../schema/events.json");
        let document: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(document, catalogue::document(), "events.json must be regenerated first");
        let mut output = String::from("// Generated from schema/events.json. Do not edit.\n");
        for entry in catalogue::ALL {
            let name = title(&entry.name);
            for field in ["input", "result"] {
                let mut schema = document["events"][entry.name.as_ref()][field].clone();
                schema["title"] = Value::String("Payload".into());
                output.push_str(&format!("export namespace {name}{} {{\n", title(field)));
                output.push_str(&bingo_schema_ts::render(&schema));
                output.push_str("}\n");
            }
            output.push_str(&format!("export type {name} = {name}Input.Payload;\n"));
        }
        output.push_str("export type Turn = TurnComplete;\n");
        for field in ["input", "result"] {
            output.push_str(&format!("export interface Event{}s {{\n", title(field)));
            for entry in catalogue::ALL {
                output.push_str(&format!(
                    "  {:?}: {}{}.Payload;\n",
                    entry.name,
                    title(&entry.name),
                    title(field)
                ));
            }
            output.push_str("}\n");
        }
        output
    }

    #[test]
    fn committed_events_are_generated() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../sdk/typescript/src/events.ts");
        let generated = rendered();
        if std::env::var_os("BINGO_UPDATE_SCHEMA").is_some() {
            std::fs::write(path, generated).unwrap();
        } else {
            assert_eq!(
                std::fs::read_to_string(path).unwrap_or_default(),
                generated,
                "events.ts drift: regenerate with BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc events_ts::"
            );
        }
    }
}
