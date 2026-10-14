use super::*;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn only_credential_free_page_fetches_follow_foreign_redirects() {
    let first = MockServer::start().await;
    let second = MockServer::start().await;
    Mock::given(wiremock::matchers::any())
        .respond_with(ResponseTemplate::new(302).insert_header("Location", second.uri()))
        .mount(&first)
        .await;
    Mock::given(wiremock::matchers::any())
        .respond_with(ResponseTemplate::new(200))
        .mount(&second)
        .await;
    assert!(
        client()
            .unwrap()
            .get(first.uri())
            .header("X-Subscription-Token", "dummy")
            .send()
            .await
            .is_err()
    );
    assert!(second.received_requests().await.unwrap().is_empty());
    assert!(
        page_client()
            .unwrap()
            .get(first.uri())
            .send()
            .await
            .unwrap()
            .status()
            .is_success()
    );
    assert_eq!(second.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn page_redirects_remain_bounded() {
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::any())
        .respond_with(ResponseTemplate::new(307).insert_header("Location", server.uri()))
        .mount(&server)
        .await;
    assert!(
        page_client()
            .unwrap()
            .get(server.uri())
            .send()
            .await
            .is_err()
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 11);
}
