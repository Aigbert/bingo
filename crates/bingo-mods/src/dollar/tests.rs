use super::nouns::{CALLS, Call, NOUNS, calls};
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
            Door::Face | Door::Service(_) => assert!(
                bingo_sdk::catalogue::find(&name).is_none(),
                "face duplicates {name}"
            ),
        }
    }
}

const IMPORTS: &str = "import type { Callback } from './callbacks.ts';\nimport type { AtSite, CommandRegistration, ProcessHandle } from './dollar.ts';\nimport type { ElementTable, OpenPane } from './ui.ts';\nimport type * as Wire from './wire.ts';\n";

fn nouns_interface() -> String {
    let mut output = String::from(
        "/** The `$` nouns: what each is, with an example. */\nexport interface DollarNouns {\n",
    );
    for noun in NOUNS {
        let text = format!("{}\n\n@example\n{}", noun.doc, noun.example);
        output.push_str(&bingo_schema_ts::doc(&text, "  "));
        let readonly = if noun.name == "plugin" {
            "readonly "
        } else {
            ""
        };
        output.push_str(&format!(
            "  {readonly}{}: unknown;\n",
            Value::from(noun.name)
        ));
    }
    output.push_str("}\n");
    output
}

fn calls_interface() -> String {
    let mut output = String::from(
        "/** Author signatures where the SDK wraps a row, and calls no row has. */\nexport interface DollarCalls {\n",
    );
    let mut nouns: Vec<&str> = CALLS.iter().map(|call| call.noun).collect();
    nouns.dedup();
    for noun in nouns {
        output.push_str(&format!("  {}: {{\n", Value::from(noun)));
        for call in calls(noun) {
            output.push_str(&bingo_schema_ts::doc(&describe(call), "    "));
            output.push_str(&format!("    {};\n", call.signature));
        }
        output.push_str("  };\n");
    }
    output.push_str("}\n");
    output
}

fn describe(call: &Call) -> String {
    let row = ROWS
        .iter()
        .find(|row| row.noun == call.noun && row.method == call.method);
    match (row, call.doc) {
        (Some(row), None) => row.doc.trim().to_string(),
        (None, Some(doc)) => doc.to_string(),
        _ => panic!("{}.{} needs exactly one doc", call.noun, call.method),
    }
}

#[test]
fn every_noun_has_a_doc_and_an_example_and_every_call_one_doc() {
    let mut nouns: Vec<&str> = ROWS.iter().map(|row| row.noun).collect();
    nouns.extend(CALLS.iter().map(|call| call.noun));
    nouns.push("plugin");
    for noun in nouns {
        let found = NOUNS.iter().find(|entry| entry.name == noun);
        let entry = found.unwrap_or_else(|| panic!("NOUNS lacks {noun}"));
        assert!(!entry.doc.trim().is_empty() && !entry.example.trim().is_empty());
        assert!(entry.example.contains(&format!("$.{noun}")), "{noun}");
    }
    for call in CALLS {
        assert!(!describe(call).is_empty());
        assert!(call.signature.starts_with(&format!("{}(", call.method)));
    }
    for row in ROWS {
        assert!(!row.doc.trim().is_empty(), "{}.{}", row.noun, row.method);
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
        for (fields, side) in [(&mut inputs, "input"), (&mut results, "result")] {
            let mut field = match &method[side] {
                Value::Bool(true) => serde_json::json!({}),
                other => other.clone(),
            };
            field["description"] = method["description"].clone();
            fields.insert(name.clone(), field);
        }
    }
    for (name, fields) in [("DollarInputs", inputs), ("DollarResults", results)] {
        schema["$defs"][name] = serde_json::json!({"type":"object", "properties":fields,
            "required":fields.keys().collect::<Vec<_>>(), "additionalProperties":false});
    }
    committed(
        "schema/dollar.json",
        format!("{}\n", serde_json::to_string_pretty(&schema).unwrap()),
    );
    let generated = bingo_schema_ts::render(&schema);
    let (header, body) = generated.split_once('\n').unwrap();
    let mut rendered = format!("{header}\n{IMPORTS}{body}");
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
                    .filter(|field| input["properties"].get(*field).is_some()
                        || (*field == "session" && method["door"]["owner"] == true))
                    .collect::<Vec<_>>()
            );
            (name.clone(), route)
        })
        .collect();
    rendered.push_str(&format!(
        "export const dollarRoutes = {} as const;\n",
        serde_json::to_string_pretty(&routes).unwrap()
    ));
    rendered.push_str(&nouns_interface());
    rendered.push_str(&calls_interface());
    committed("sdk/typescript/src/dollar-types.ts", rendered);
}

#[test]
fn world_methods_have_one_declared_door() {
    for (noun, methods) in [
        (
            "fs",
            &["read", "write", "list", "exists", "stat", "ancestors"][..],
        ),
        ("http", &["fetch"][..]),
        ("process", &["run", "spawn", "read", "write", "kill"][..]),
        ("env", &["get", "set"][..]),
        ("mcp", &["call", "connect"][..]),
        ("agent", &["spawn", "list", "register"][..]),
    ] {
        for method in methods {
            assert_eq!(
                ROWS.iter()
                    .filter(|row| row.noun == noun && row.method == *method)
                    .count(),
                1,
                "{noun}.{method}"
            );
        }
    }
}
