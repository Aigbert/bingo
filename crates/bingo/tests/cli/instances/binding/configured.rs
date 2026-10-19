use super::*;

#[tokio::test]
async fn project_override_cannot_redirect_anthropic_key() {
    overridden("anthropic", "work", "baseUrl").await;
}

async fn configured_override(family: &str, environment: bool, local: bool) {
    let foreign = endpoint_for("configured-test-key").await;
    let home = tempfile::tempdir().unwrap();
    let name = if environment { family } else { "work" };
    if !environment {
        user_settings(
            home.path(),
            json!({family:{"instances":{name:{"apiKey":"configured-test-key"}}}}),
        );
    }
    let endpoint = json!({"baseUrl":foreign.uri()});
    let overlay = if environment {
        json!({family:endpoint})
    } else {
        json!({family:{"instances":{name:endpoint}}})
    };
    let root = project(home.path(), overlay);
    let dir = root.join(".bingo");
    let mut file = dir.join("settings.toml");
    if local {
        std::fs::rename(dir.join("settings.json"), dir.join("settings.local.json")).unwrap();
        file = dir.join("settings.local.toml");
    }
    let mut cmd = at(home.path(), &root, name);
    if environment {
        cmd.env(
            format!("{}_API_KEY", family.to_uppercase()),
            "configured-test-key",
        );
    }
    let out = tokio::task::spawn_blocking(move || run(&mut cmd))
        .await
        .unwrap();
    assert!(
        foreign.received_requests().await.unwrap().is_empty(),
        "key reached override"
    );
    let notice = stderr(&out);
    assert!(notice.contains("AUTH_REQUIRED"), "{notice}");
    assert!(notice.contains(&file.display().to_string()), "{notice}");
    assert!(!notice.contains("configured-test-key"));
}

#[tokio::test]
async fn repository_addresses_do_not_receive_environment_keys() {
    for family in ["openai", "anthropic"] {
        for local in [false, true] {
            configured_override(family, true, local).await;
        }
    }
}

#[tokio::test]
async fn repository_addresses_do_not_receive_user_settings_keys() {
    for family in ["openai", "anthropic"] {
        for local in [false, true] {
            configured_override(family, false, local).await;
        }
    }
}

#[tokio::test]
async fn environment_address_is_used_for_legacy_adoption() {
    let server = endpoint_for("legacy-test-key").await;
    let home = tempfile::tempdir().unwrap();
    write_auth(
        home.path(),
        json!({"openai":{"type":"api","key":"legacy-test-key"}}),
    );
    let mut cmd = at(home.path(), home.path(), "openai");
    cmd.env("OPENAI_BASE_URL", server.uri());
    let out = tokio::task::spawn_blocking(move || run(&mut cmd))
        .await
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!stderr(&out).contains("not the one its key"));
    assert_eq!(stored(home.path())["openai"]["origin"], server.uri());
}

#[test]
fn blank_address_adopts_the_default_origin() {
    let home = tempfile::tempdir().unwrap();
    user_settings(
        home.path(),
        json!({"openai":{"instances":{"work":{"baseUrl":" "}}}}),
    );
    // Expired, unusable token entry prevents HTTP while exercising metadata adoption.
    write_auth(
        home.path(),
        json!({"work":{"type":"oauth","access":"dummy","refresh":"dummy","expires":0}}),
    );
    let out = run(&mut at(home.path(), home.path(), "work"));
    assert!(!out.status.success());
    assert_eq!(
        stored(home.path())["work"]["origin"],
        "https://api.openai.com:443"
    );
}

#[tokio::test]
async fn keys_can_reach_the_origin_declared_beside_them() {
    for environment in [false, true] {
        let server = endpoint_for("configured-test-key").await;
        let home = tempfile::tempdir().unwrap();
        let root = project(
            home.path(),
            if environment {
                json!({})
            } else {
                json!({"openai":{"baseUrl":server.uri(),"apiKey":"configured-test-key"}})
            },
        );
        let mut cmd = at(home.path(), &root, "openai");
        if environment {
            cmd.env("OPENAI_API_KEY", "configured-test-key")
                .env("OPENAI_BASE_URL", server.uri());
        }
        let out = tokio::task::spawn_blocking(move || run(&mut cmd))
            .await
            .unwrap();
        assert!(out.status.success(), "{}", stderr(&out));
        assert!(!stderr(&out).contains("not the one its key"));
        assert!(!server.received_requests().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn anthropic_environment_address_is_used_for_legacy_adoption() {
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, path},
    };
    let server = MockServer::start().await;
    Mock::given(path("/v1/messages"))
        .and(header("x-api-key", "legacy-anthropic-dummy"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../../../../../bingo-provider-anthropic/fixtures/text.sse"),
            "text/event-stream",
        ))
        .expect(1)
        .mount(&server)
        .await;
    let home = tempfile::tempdir().unwrap();
    write_auth(
        home.path(),
        json!({"anthropic":{"type":"api","key":"legacy-anthropic-dummy"}}),
    );
    let mut cmd = at(home.path(), home.path(), "anthropic");
    cmd.env("ANTHROPIC_BASE_URL", server.uri());
    let out = tokio::task::spawn_blocking(move || run(&mut cmd))
        .await
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!stderr(&out).contains("not the one its key"));
    assert_eq!(stored(home.path())["anthropic"]["origin"], server.uri());
}
