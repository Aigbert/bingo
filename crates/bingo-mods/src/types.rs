//! Host-derived author types are written only when their bytes change.
use bingo_sdk::Catalog;
use std::path::Path;
pub fn write(_root: &Path, _events: &Catalog, _tools: &Catalog) -> Result<(), String> {
    Ok(())
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
