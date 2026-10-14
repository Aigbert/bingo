#![allow(clippy::unwrap_used, clippy::expect_used)]
use bingo_mods::testing::{Driver, Outcome, Runner};
use serde_json::{Value, json};
struct Offline;
#[async_trait::async_trait]
impl Driver for Offline {
    async fn call(&mut self, _: &str, _: Value) -> Result<Value, String> {
        Err("unexpected host call".into())
    }
}
#[tokio::test]
async fn testing_module_runs_matchers_and_reports_each_outcome() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/testing");
    let runner = Runner::load(&root, "expect.test.ts", Default::default())
        .await
        .unwrap();
    let reports = runner
        .run(
            &mut Offline,
            json!({"session":"s","plugin":{"name":"fixture","root":root}}),
        )
        .await
        .unwrap();
    runner.close().await;
    assert_eq!(reports.len(), 3);
    assert_eq!(reports[0].outcome, Outcome::Pass);
    assert!(
        matches!(&reports[1].outcome, Outcome::Fail { message, at } if message.contains("2") && at.contains("expect.test.ts:"))
    );
    assert_eq!(reports[2].outcome, Outcome::Skip);
}
