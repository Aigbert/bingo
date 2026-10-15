#![allow(clippy::unwrap_used, clippy::expect_used)]
use bingo_sdk::*;
#[test]
fn fork_summary_round_trips_and_old_summaries_default() {
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/record/compat/summary-forked.json"
    ))
    .unwrap();
    let summary: SessionSummary = serde_json::from_value(value["summary"].clone()).unwrap();
    assert_eq!(
        summary.history.first_ask.as_deref(),
        Some("Fix the stale-edit race")
    );
    assert_eq!(serde_json::to_value(&summary).unwrap(), value["summary"]);
    let link = summary.history.forked_from.unwrap();
    assert_eq!(link.session.as_str(), "ses_source");
    assert_eq!(link.turn.unwrap().as_str(), "trn_1");
    let mut old = value["summary"].clone();
    old.as_object_mut().unwrap().remove("firstAsk");
    old.as_object_mut().unwrap().remove("forkedFrom");
    let summary: SessionSummary = serde_json::from_value(old).unwrap();
    assert_eq!(summary.history.first_ask, None);
    assert_eq!(summary.history.forked_from, None);
    let spec: SessionSpec = serde_json::from_value(
        serde_json::json!({"cwd":"/tmp", "forkedFrom":{"session":"ses_source","turn":null}}),
    )
    .unwrap();
    assert!(
        serde_json::to_value(spec)
            .unwrap()
            .get("forkedFrom")
            .is_none()
    );
}
#[cfg(feature = "testing")]
#[tokio::test]
async fn hosts_without_fork_refuse_by_default() {
    let host = bingo_sdk::testing::NoHost;
    assert_eq!(
        host.fork(&SessionId::from_raw("s"), None)
            .await
            .unwrap_err()
            .code,
        ErrorCode::Internal
    );
}

#[test]
fn a_flattened_history_does_not_describe_the_whole_summary() {
    let schema = schemars::schema_for!(SessionSummary);
    assert!(schema.get("description").is_none());
    let spec = schemars::schema_for!(SessionSpec);
    assert!(spec.get("properties").unwrap().get("forkedFrom").is_none());
}
