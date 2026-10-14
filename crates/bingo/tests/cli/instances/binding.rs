//! Stored keys cannot follow repository endpoint overrides.
use super::*;
use serde_json::{Value, json};

fn user_settings(home: &std::path::Path, value: Value) {
    let dir = home.join(".bingo");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("settings.json"), value.to_string()).unwrap();
}

fn project(home: &std::path::Path, value: Value) -> std::path::PathBuf {
    let root = home.join("project");
    let dir = root.join(".bingo");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("settings.json"), value.to_string()).unwrap();
    root
}

fn at(home: &std::path::Path, root: &std::path::Path, provider: &str) -> Command {
    let mut cmd = bingo();
    cmd.envs(home_env(home)).args(["--cwd"]).arg(root).args([
        "--print",
        "--provider",
        provider,
        "--model",
        "gpt-5.4",
        "hi",
    ]);
    cmd
}

async fn overridden(family: &str, name: &str, field: &str) {
    let trusted = endpoint_for("stored-test-key").await;
    let foreign = endpoint_for("stored-test-key").await;
    let home = tempfile::tempdir().unwrap();
    let mut endpoint = json!({"baseUrl": trusted.uri()});
    endpoint[field] = json!(trusted.uri());
    let user = if name == family {
        json!({family: endpoint})
    } else {
        json!({family: {"instances": {name: endpoint}}})
    };
    user_settings(home.path(), user);
    let override_value = json!({field: foreign.uri()});
    let overlay = if name == family {
        json!({family: override_value})
    } else {
        json!({family: {"instances": {name: override_value}}})
    };
    let root = project(home.path(), overlay);
    let mut entry = if field == "issuer" {
        json!({"type": "oauth", "access": "stored-test-key", "refresh": "refresh-test-key", "expires": 1})
    } else {
        json!({"type": "api", "key": "stored-test-key"})
    };
    entry["owner"] = json!(family);
    entry["origin"] = json!(trusted.uri());
    write_auth(home.path(), json!({name: entry}));
    let h = home.path().to_path_buf();
    let n = name.to_string();
    let r = root.clone();
    let out = tokio::task::spawn_blocking(move || run(&mut at(&h, &r, &n)))
        .await
        .unwrap();
    assert!(
        foreign.received_requests().await.unwrap().is_empty(),
        "credential reached override"
    );
    let notice = stderr(&out);
    assert!(notice.contains("AUTH_REQUIRED"), "{notice}");
    assert!(
        notice.contains("not the one its key was entered for"),
        "{notice}"
    );
    assert!(
        notice.contains(&root.join(".bingo/settings.json").display().to_string()),
        "{notice}"
    );
    assert!(!notice.contains("stored-test-key"), "{notice}");
}

#[tokio::test]
async fn project_override_cannot_redirect_named_key() {
    overridden("openai", "work", "baseUrl").await;
}

#[tokio::test]
async fn project_override_cannot_redirect_default_key() {
    overridden("openai", "openai", "baseUrl").await;
}

#[tokio::test]
async fn project_override_cannot_redirect_oauth_issuer() {
    overridden("codex", "work", "issuer").await;
}

#[tokio::test]
async fn user_layer_adopts_a_legacy_named_key() {
    let server = endpoint_for("legacy-test-key").await;
    let home = tempfile::tempdir().unwrap();
    user_settings(
        home.path(),
        json!({"openai": {"instances": {"work": {"baseUrl": server.uri()}}}}),
    );
    write_auth(
        home.path(),
        json!({"work": {"type": "api", "key": "legacy-test-key"}}),
    );
    let h = home.path().to_path_buf();
    let out = tokio::task::spawn_blocking(move || run(&mut at(&h, &h, "work")))
        .await
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(stored(home.path())["work"]["owner"], "openai");
    assert_eq!(stored(home.path())["work"]["origin"], server.uri());
}

#[tokio::test]
async fn legacy_adoption_uses_user_origin_before_a_project_override() {
    let trusted = endpoint_for("legacy-test-key").await;
    let foreign = endpoint_for("legacy-test-key").await;
    let home = tempfile::tempdir().unwrap();
    user_settings(
        home.path(),
        json!({"openai":{"instances":{"work":{"baseUrl":trusted.uri()}}}}),
    );
    let root = project(
        home.path(),
        json!({"openai":{"instances":{"work":{"baseUrl":foreign.uri()}}}}),
    );
    write_auth(
        home.path(),
        json!({"work":{"type":"api","key":"legacy-test-key"}}),
    );
    let h = home.path().to_path_buf();
    let out = tokio::task::spawn_blocking(move || run(&mut at(&h, &root, "work")))
        .await
        .unwrap();
    assert!(stderr(&out).contains("AUTH_REQUIRED"), "{}", stderr(&out));
    assert!(foreign.received_requests().await.unwrap().is_empty());
    assert_eq!(stored(home.path())["work"]["origin"], trusted.uri());
}

#[test]
fn repository_and_local_layers_cannot_adopt_legacy_entries() {
    for filename in ["settings.json", "settings.local.json"] {
        let home = tempfile::tempdir().unwrap();
        let root = project(home.path(), json!({}));
        std::fs::write(
            root.join(".bingo").join(filename),
            json!({
                "openai":{"instances":{"work":{}}}
            })
            .to_string(),
        )
        .unwrap();
        write_auth(
            home.path(),
            json!({"work":{"type":"api","key":"legacy-test-key"}}),
        );
        let out = run(&mut at(home.path(), &root, "work"));
        assert!(!out.status.success());
        assert!(stored(home.path())["work"].get("owner").is_none());
        assert!(stored(home.path())["work"].get("origin").is_none());
    }
}

#[test]
fn ambiguous_user_declarations_cannot_adopt_legacy_entries() {
    let home = tempfile::tempdir().unwrap();
    user_settings(
        home.path(),
        json!({
            "openai":{"instances":{"work":{}}}, "anthropic":{"instances":{"work":{}}}
        }),
    );
    write_auth(
        home.path(),
        json!({"work":{"type":"api","key":"legacy-test-key"}}),
    );
    let out = run(&mut at(home.path(), home.path(), "work"));
    assert!(!out.status.success());
    assert!(stored(home.path())["work"].get("owner").is_none());
}
#[tokio::test]
async fn foreign_owner_notice_names_both_families_and_the_override() {
    let server = endpoint_for("owned-test-key").await;
    let home = tempfile::tempdir().unwrap();
    user_settings(
        home.path(),
        json!({"anthropic":{"instances":{"work":{"baseUrl":server.uri()}}}}),
    );
    let root = project(
        home.path(),
        json!({"openai":{"instances":{"work":{"baseUrl":server.uri()}}}}),
    );
    write_auth(
        home.path(),
        json!({"work":{
            "type":"api", "key":"owned-test-key", "owner":"anthropic", "origin":server.uri()
        }}),
    );
    let h = home.path().to_path_buf();
    let r = root.clone();
    let out = tokio::task::spawn_blocking(move || run(&mut at(&h, &r, "openai")))
        .await
        .unwrap();
    let notice = stderr(&out);
    assert!(
        notice.contains("anthropic") && notice.contains("openai"),
        "{notice}"
    );
    assert!(
        notice.contains(&root.join(".bingo/settings.json").display().to_string()),
        "{notice}"
    );
    assert!(notice.contains("rename or delete"), "{notice}");
    assert!(!notice.contains("/provider remove work"), "{notice}");
    assert_eq!(stored(home.path())["work"]["owner"], "anthropic");
}
