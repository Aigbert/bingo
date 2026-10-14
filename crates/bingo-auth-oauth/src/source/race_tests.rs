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
