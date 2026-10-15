use bingo_sdk::*;
use serde_json::json;
pub fn summary(id: &str, title: Option<&str>) -> SessionSummary {
    serde_json::from_value(json!({"id":id,"title":title,"cwd":"/work","createdAt":"2026-10-15T10:00:00Z","updatedAt":"2026-10-15T10:00:00Z"})).unwrap()
}
pub fn props() -> component::SessionsProps {
    serde_json::from_value(json!({"session":"current","cwd":"/work","query":"","scope":"here","limit":50,"sessions":[],"now":"2026-10-15T12:00:00Z","action":{"name":"client","args":{}}})).unwrap()
}
