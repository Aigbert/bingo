mod support;
use serde_json::json;
use support::Fixture;

#[tokio::test]
async fn a_reentrant_call_skips_only_its_calling_hook_and_says_why_once() {
    let mut fixture =
        Fixture::new(include_str!("mods/reentrant.ts"), &["fixture"], json!({})).await;
    let result = fixture.call("fixture", "echo", json!({"value":1})).await;
    assert_eq!(result, json!({"value":1,"other":1}));
    let notices = fixture.notices().await;
    assert_eq!(notices.len(), 1, "{notices:?}");
    assert!(
        notices[0].contains("fixture.echo") && notices[0].contains("re-entry"),
        "{notices:?}"
    );
    fixture.host.shutdown().await;
}

#[tokio::test]
async fn throw_loop_and_hog_fail_open_with_one_named_notice() {
    for (source, method, options, kind) in [
        (include_str!("mods/thrower.ts"), "call", json!({}), "throw"),
        (
            include_str!("mods/loop.ts"),
            "check",
            json!({"mods":{"hookMs":50}}),
            "timeout",
        ),
        (
            include_str!("mods/hog.ts"),
            "call",
            json!({"mods":{"memoryMiB":4}}),
            "memory",
        ),
    ] {
        let mut fixture = Fixture::new(&source.replace("tool.", "fixture."), &[], options).await;
        let input = json!({"tool":"Read", "tool_use_id":"1"});
        assert_eq!(fixture.call("fixture", method, input.clone()).await, input);
        let notices = fixture.notices().await;
        assert_eq!(notices.len(), 1, "{notices:?}");
        assert!(
            notices[0].contains("fixture")
                && notices[0].contains(&format!("fixture.{method}"))
                && notices[0].contains(kind),
            "{notices:?}"
        );
        fixture.host.shutdown().await;
    }
}

#[tokio::test]
async fn an_escaper_load_failure_is_visible_once() {
    let mut fixture = Fixture::new(include_str!("mods/escaper.ts"), &[], json!({})).await;
    let notices = fixture.notices().await;
    assert_eq!(notices.len(), 1, "{notices:?}");
    assert!(
        notices[0].contains("fixture") && notices[0].contains("main.ts:"),
        "{notices:?}"
    );
    fixture.host.shutdown().await;
}
