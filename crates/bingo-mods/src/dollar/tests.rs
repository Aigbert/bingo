use super::table::{self, Door, ROWS};
use serde_json::Value;

#[test]
fn each_method_has_one_door_in_the_catalogue_namespace() {
    let mut names = std::collections::BTreeSet::new();
    for row in ROWS {
        let name = format!("{}.{}", row.noun, row.method);
        assert!(names.insert(name.clone()), "duplicate row: {name}");
        match row.door {
            Door::Kernel(entry) => assert_eq!(entry.name, name),
            Door::Face => assert!(
                bingo_sdk::catalogue::find(&name).is_none(),
                "face duplicates {name}"
            ),
        }
    }
}

fn committed(path: &str, rendered: String) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path);
    if std::env::var_os("BINGO_UPDATE_SCHEMA").is_some() {
        std::fs::write(path, rendered).unwrap();
    } else {
        assert_eq!(
            std::fs::read_to_string(path).unwrap_or_default(),
            rendered,
            "dollar contract drift"
        );
    }
}

#[test]
fn schema_and_types_are_generated_from_the_same_rows() {
    let mut schema = table::document();
    let mut inputs = serde_json::Map::new();
    let mut results = serde_json::Map::new();
    for (name, method) in schema["methods"].as_object().unwrap() {
        inputs.insert(name.clone(), method["input"].clone());
        results.insert(name.clone(), method["result"].clone());
    }
    for (name, fields) in [("DollarInputs", inputs), ("DollarResults", results)] {
        schema["$defs"][name] = serde_json::json!({"type":"object", "properties":fields,
            "required":fields.keys().collect::<Vec<_>>(), "additionalProperties":false});
    }
    committed(
        "schema/dollar.json",
        format!("{}\n", serde_json::to_string_pretty(&schema).unwrap()),
    );
    let mut rendered = bingo_schema_ts::render(&schema);
    let routes: serde_json::Map<String, Value> = schema["methods"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, method)| {
            let mut route = method["door"].clone();
            let input = method["input"]["$ref"]
                .as_str()
                .and_then(|reference| schema.pointer(&reference[1..]))
                .unwrap_or(&method["input"]);
            route["site"] = serde_json::json!(
                ["session", "turn"]
                    .into_iter()
                    .filter(|field| input["properties"].get(*field).is_some())
                    .collect::<Vec<_>>()
            );
            (name.clone(), route)
        })
        .collect();
    rendered.push_str(&format!(
        "export const dollarRoutes = {} as const;\n",
        serde_json::to_string_pretty(&routes).unwrap()
    ));
    committed("sdk/typescript/src/dollar-types.ts", rendered);
}
