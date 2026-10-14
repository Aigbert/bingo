use super::*;
use serde_json::json;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::path};

#[tokio::test]
async fn late_refresh_cannot_replace_or_remove_a_new_resource_credential() {
    for expired in [false, true] {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let store = CredentialStore::new(dir.path().into())
            .owned_by("codex")
            .at_origin(&server.uri(), "user settings");
        let old = store.at_resource("https://old.example", "user settings");
        let new = store.at_resource("https://new.example", "user settings");
        old.write(
            "work",
            Entry::OAuth {
                access: "old-access".into(),
                refresh: "old-refresh".into(),
                expires: 1,
                account_id: None,
            },
        )
        .unwrap();
        let replacement = Entry::OAuth {
            access: "new-access".into(),
            refresh: "new-refresh".into(),
            expires: unix_now() + 3600,
            account_id: None,
        };
        let writer = new.clone();
        let entry = replacement.clone();
        Mock::given(path("/oauth/token"))
            .respond_with(move |_: &wiremock::Request| {
                // Replace the entry after the old exchange starts, before its reply.
                writer.write("work", entry.clone()).unwrap();
                if expired {
                    ResponseTemplate::new(400)
                        .set_body_json(json!({"error":"refresh_token_expired"}))
                } else {
                    ResponseTemplate::new(200)
                        .set_body_json(json!({"access_token":"late-access", "expires_in":3600}))
                }
            })
            .expect(1)
            .mount(&server)
            .await;
        let source = TokenSource::new(
            "work",
            crate::issuer::tests::issuer(&server.uri()),
            Arc::new(old),
            bingo_http::client(bingo_http::Bounds::CALL).unwrap(),
        );
        assert!(source.access_token().await.is_err());
        assert_eq!(new.read("work").unwrap(), Some(replacement));
    }
}

fn credential(access: &str, expires: i64) -> Entry {
    Entry::OAuth {
        access: access.into(),
        refresh: "dummy-refresh".into(),
        expires,
        account_id: None,
    }
}

async fn replaced_during_refresh(
    replacement: Option<Entry>,
    expired: bool,
) -> Result<String, AuthError> {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(
        CredentialStore::new(dir.path().into())
            .owned_by("family")
            .at_origin(&server.uri(), "user settings")
            .at_resource("https://resource.example", "user settings"),
    );
    store.write("work", credential("old", 1)).unwrap();
    let writer = store.clone();
    let entry = replacement.clone();
    Mock::given(path("/oauth/token"))
        .respond_with(move |_: &wiremock::Request| {
            match &entry {
                Some(entry) => writer.write("work", entry.clone()).unwrap(),
                None => writer.remove("work").unwrap(),
            }
            if expired {
                ResponseTemplate::new(400).set_body_json(json!({"error":"refresh_token_expired"}))
            } else {
                ResponseTemplate::new(200)
                    .set_body_json(json!({"access_token":"late", "expires_in":3600}))
            }
        })
        .expect(1)
        .mount(&server)
        .await;
    let source = TokenSource::new(
        "work",
        crate::issuer::tests::issuer(&server.uri()),
        store.clone(),
        bingo_http::client(bingo_http::Bounds::CALL).unwrap(),
    );
    let result = source.access_token().await;
    assert_eq!(store.read("work").unwrap(), replacement);
    if result.is_ok() {
        assert!(matches!(source.status(), Status::SignedIn { .. }));
        assert!(
            matches!(source.cached().0, Some(Cached::Tokens(tokens)) if tokens.access == "new")
                || matches!(source.cached().0, Some(Cached::Key(key)) if key == "new")
        );
    }
    result
}

#[tokio::test]
async fn a_late_refresh_serves_a_usable_same_binding_replacement() {
    for expired in [false, true] {
        for replacement in [
            credential("new", unix_now() + 3600),
            credential("new", unix_now() + 120),
            Entry::Api { key: "new".into() },
        ] {
            assert_eq!(
                replaced_during_refresh(Some(replacement), expired)
                    .await
                    .unwrap(),
                "new"
            );
        }
    }
}

#[tokio::test]
async fn a_late_refresh_refuses_an_unusable_replacement() {
    for expired in [false, true] {
        for replacement in [
            None,
            Some(credential("new", 1)),
            Some(credential("new", 0)),
            Some(credential("", unix_now() + 3600)),
            Some(Entry::Api { key: String::new() }),
        ] {
            assert!(replaced_during_refresh(replacement, expired).await.is_err());
        }
    }
}
