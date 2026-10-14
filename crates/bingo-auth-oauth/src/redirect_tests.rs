use crate::{exchange, issuer::tests::issuer};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

#[tokio::test]
async fn redirects_cannot_repost_refresh_tokens_to_another_origin() {
    for form_encoded in [false, true] {
        for status in [307, 308] {
            let first = MockServer::start().await;
            let foreign = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/oauth/token"))
                .respond_with(
                    ResponseTemplate::new(status)
                        .insert_header("location", format!("{}/stolen", foreign.uri())),
                )
                .mount(&first)
                .await;
            let mut issuer = issuer(&first.uri());
            issuer.form_encoded = form_encoded;
            let http = bingo_http::client(bingo_http::Bounds::CALL).unwrap();
            assert!(
                exchange::refresh(&http, &issuer, "dummy-refresh")
                    .await
                    .is_err()
            );
            assert!(
                foreign.received_requests().await.unwrap().is_empty(),
                "refresh followed {status}"
            );
        }
    }
}

#[tokio::test]
async fn redirects_within_the_origin_repost_the_refresh_form() {
    let server = MockServer::start().await;
    Mock::given(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(307).insert_header("location", "/token"))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"access_token":"renewed"})),
        )
        .mount(&server)
        .await;
    let mut issuer = issuer(&server.uri());
    issuer.form_encoded = true;
    let http = bingo_http::client(bingo_http::Bounds::CALL).unwrap();
    assert_eq!(
        exchange::refresh(&http, &issuer, "dummy-refresh")
            .await
            .unwrap()["access_token"],
        "renewed"
    );
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].body, requests[1].body);
}
