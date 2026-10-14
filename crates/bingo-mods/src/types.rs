//! Host-derived author types are written only when their bytes change.
use bingo_sdk::Catalog;
use std::path::Path;
mod files;
mod render;
pub fn write(root: &Path, events: &Catalog, tools: &Catalog) -> Result<(), String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    for (name, text) in sources(events, tools) {
        files::changed(&root, &format!(".bingo/{name}"), text.as_bytes())?;
    }
    if !root.join("tsconfig.json").exists() {
        files::changed(&root,"tsconfig.json", b"{\"extends\":\"./.bingo/tsconfig.json\",\"include\":[\"**/*.ts\",\"**/*.tsx\",\".bingo/types/**/*.ts\"]}\n")?;
    }
    Ok(())
}
fn sources(events: &Catalog, tools: &Catalog) -> Vec<(&'static str, String)> {
    let mut files = vec![
        ("types/bingo/index.d.ts",include_str!("types/index.d.ts").into()),
        ("types/bingo/testing.d.ts",include_str!("types/testing.d.ts").replace("./bingo/","./")),
        ("types/bingo/elements.d.ts",render::elements()),
        ("types/bingo/events.ts",render::events(events)),
        ("types/bingo/services.ts",render::services(events)),
        ("types/bingo-tools/index.d.ts",render::tools(tools)),
        ("types/bingo/rpc.ts","export interface Rpc {request<T>(method: string, params: unknown): Promise<T>}\n".into()),
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
        write(root.path(), &events, &tools).unwrap();
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
        write(root.path(), &events, &tools).unwrap();
        assert_eq!(
            std::fs::metadata(&event_path).unwrap().modified().unwrap(),
            time
        );
        assert_eq!(
            std::fs::read_to_string(root.path().join("tsconfig.json")).unwrap(),
            "{\"author\":true}"
        );
    }
}
