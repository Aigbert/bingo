#![allow(clippy::unwrap_used)]
use bingo_mods::engine::{Engine, Limits};
use serde_json::json;
use std::path::Path;

#[tokio::test]
async fn a_fixture_registers_on_its_own_runtime() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/mods");
    Engine::start(&root, "hello.ts", json!({}), Limits::default()).await.unwrap();
}
