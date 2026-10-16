//! Host-derived author types, written when an author asks and only when
//! their bytes change. A run never writes them: the runtime embeds the API.
use bingo_sdk::Catalog;
use std::path::{Path, PathBuf};
mod elements;
mod examples;
mod files;
mod reference;
mod render;
/// The files whose bytes changed, in writing order.
pub fn write(root: &Path, events: &Catalog, tools: &Catalog) -> Result<Vec<PathBuf>, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut written = Vec::new();
    for (name, text) in sources(events, tools) {
        written.extend(files::changed(
            &root,
            &format!(".bingo/{name}"),
            text.as_bytes(),
        )?);
    }
    if !root.join("tsconfig.json").exists() {
        written.extend(files::changed(&root,"tsconfig.json", b"{\"extends\":\"./.bingo/tsconfig.json\",\"include\":[\"**/*.ts\",\"**/*.tsx\",\".bingo/types/**/*.ts\"]}\n")?);
    }
    Ok(written)
}
fn sources(events: &Catalog, tools: &Catalog) -> Vec<(String, String)> {
    let mut files = vec![
        ("types/bingo/index.d.ts",include_str!("types/index.d.ts").into()),
        ("types/bingo/testing.d.ts",include_str!("types/testing.d.ts").replace("./bingo/","./")),
        ("types/bingo/elements.d.ts",elements::declarations()),
        ("types/bingo/events.ts",render::events(events)),
        ("types/bingo/services.ts",render::services(events)),
        ("types/bingo-tools/index.d.ts",render::tools(tools)),
        ("types/bingo/rpc.ts","export interface Rpc {request<T>(method: string, params: unknown): Promise<T>}\n".into()),
        ("reference.md",reference::reference(events)),
        (".gitignore","*\n".into()),
        ("tsconfig.json",serde_json::json!({"compilerOptions":{"target":"ES2023","module":"ESNext","moduleResolution":"Bundler","allowImportingTsExtensions":true,"noEmit":true,"skipLibCheck":true,"lib":["ES2023"],"jsx":"react","jsxFactory":"h","jsxFragmentFactory":"Fragment","baseUrl":".","paths":{"bingo":["types/bingo/index.d.ts"],"bingo/testing":["types/bingo/testing.d.ts"],"bingo-tools":["types/bingo-tools/index.d.ts"]}}}).to_string()),
    ];
    macro_rules! source {
        ($name:literal) => {
            files.push((
                concat!("types/bingo/", $name),
                include_str!(concat!("../../../sdk/typescript/src/", $name)).into(),
            ));
        };
    }
    source!("on.ts");
    source!("state.ts");
    files.push(("types/bingo/dollar.ts",include_str!("../../../sdk/typescript/src/dollar.ts").replace("export type Dollar =", "type HostDollar =") + "\nexport type Dollar = Omit<HostDollar,keyof import('./services.ts').Services> & import('./services.ts').Services;\n"));
    source!("dollar-types.ts");
    source!("callbacks.ts");
    source!("wire.ts");
    let table = bingo_sdk::view::table::ELEMENTS
        .iter()
        .map(|element| {
            format!(
                "{}:{}",
                serde_json::json!(element.kind),
                serde_json::json!(element.kind)
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    files.push((
        "types/bingo/ui.ts",
        include_str!("../../../sdk/typescript/src/ui.ts")
            .replace("Record<string,string>", &format!("{{{table}}}")),
    ));
    files
        .into_iter()
        .map(|(name, text)| (name.to_owned(), text))
        .chain(examples::laid().map(|(name, text)| (name, text.to_owned())))
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::{CatalogEntry, CatalogKind};
    use serde_json::json;
    #[test]
    fn writes_event_and_tool_types_once_and_keeps_the_authors_tsconfig() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("tsconfig.json"), "{\"author\":true}").unwrap();
        let events = Catalog {
            kind: CatalogKind::Plugins,
            entries: vec![CatalogEntry {
                id: "kit.echo".into(),
                label: "kit.echo".into(),
                meta: json!({"input":{"type":"string"},"result":{"type":"number"}}),
            }],
        };
        let tools = Catalog {
            kind: CatalogKind::Tools,
            entries: vec![CatalogEntry {
                id: "Echo".into(),
                label: "Echo".into(),
                meta: json!({"inputSchema":{"type":"object","properties":{"text":{"type":"string"}},"required":["text"]}}),
            }],
        };
        assert!(!write(root.path(), &events, &tools).unwrap().is_empty());
        let event_path = root.path().join(".bingo/types/bingo/events.ts");
        let text = std::fs::read_to_string(&event_path).unwrap();
        assert!(
            text.contains("KitEcho")
                && text.contains("Payload = string")
                && text.contains("Payload = number")
        );
        assert!(
            std::fs::read_to_string(root.path().join(".bingo/types/bingo-tools/index.d.ts"))
                .unwrap()
                .contains("Echo")
        );
        let time = std::fs::metadata(&event_path).unwrap().modified().unwrap();
        assert_eq!(
            write(root.path(), &events, &tools).unwrap(),
            Vec::<PathBuf>::new()
        );
        assert_eq!(
            std::fs::metadata(&event_path).unwrap().modified().unwrap(),
            time
        );
        assert_eq!(
            std::fs::read_to_string(root.path().join("tsconfig.json")).unwrap(),
            "{\"author\":true}"
        );
    }

    /// Every line of `text` is the content of a JSDoc line in `laid`.
    fn carried(laid: &str, text: &str) -> bool {
        let jsdoc: std::collections::HashSet<&str> = laid
            .lines()
            .map(str::trim)
            .filter_map(|line| line.strip_prefix("/**").or_else(|| line.strip_prefix('*')))
            .map(|line| line.strip_suffix("*/").unwrap_or(line).trim())
            .collect();
        text.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .all(|line| jsdoc.contains(line))
    }

    #[test]
    fn the_laid_types_carry_every_event_and_method_doc_as_jsdoc() {
        use crate::dollar::nouns::{CALLS, NOUNS};
        use crate::dollar::table::ROWS;
        let root = tempfile::tempdir().unwrap();
        let tools = Catalog {
            kind: CatalogKind::Tools,
            entries: Vec::new(),
        };
        write(root.path(), &reference::tests::kernel_events(), &tools).unwrap();
        let bingo = root.path().join(".bingo/types/bingo");
        let laid: String = std::fs::read_dir(&bingo)
            .unwrap()
            .map(|file| std::fs::read_to_string(file.unwrap().path()).unwrap())
            .collect();
        for entry in bingo_sdk::catalogue::ALL {
            let doc = entry.description().unwrap();
            assert!(carried(&laid, &doc), "{}", entry.name);
        }
        for row in ROWS {
            assert!(carried(&laid, row.doc), "{}.{}", row.noun, row.method);
        }
        for call in CALLS.iter().filter_map(|call| call.doc) {
            assert!(carried(&laid, call), "{call}");
        }
        for noun in NOUNS {
            assert!(carried(&laid, noun.doc), "{}", noun.name);
            assert!(carried(&laid, noun.example), "{}", noun.name);
        }
    }
}
