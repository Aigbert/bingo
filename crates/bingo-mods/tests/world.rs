#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use serde_json::json;
use support::Fixture;

#[tokio::test]
async fn filesystem_face_reads_a_scoped_file_through_the_module() {
    let fixture = Fixture::new(include_str!("mods/world/files.ts"), &["fs"], json!({})).await;
    let path = fixture.home.path().join("sample.txt");
    std::fs::write(&path, "world").unwrap();
    let answer = fixture.call("world", "echo", json!({"path":path})).await;
    assert_eq!(answer, json!({"value":"world"}));
    fixture.host.shutdown().await;
}
