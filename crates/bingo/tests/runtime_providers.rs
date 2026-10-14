//! Runtime provider management through the real stdio RPC process, with local endpoints.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use bingo_sdk::*;
use futures::StreamExt;
use serde_json::json;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};
mod support;
use support::{LIMIT, Server, ack_for, create, ready, until_completed, who};
const KEY: &str = "runtime-secret-never-journaled";
const IDLE: &str = r#"{"responses":[{"steps":[{"text":"unused"}]}]}"#;

async fn endpoint(text: &str) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"data":[{"id":"local-model"}]})),
        )
        .mount(&server)
        .await;
    let body = include_str!("../../bingo-provider-openai/fixtures/text.sse").replace("Hello", text);
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
        .mount(&server)
        .await;
    server
}

async fn command(session: &mut Attachment, text: &str) -> IntentOutcome {
    let intent = IntentId::mint();
    session
        .handle
        .submit(intent.clone(), Input::text(text, Origin::surface("test")));
    ack_for(session, &intent).await
}
async fn applied(session: &mut Attachment, text: &str) {
    assert!(
        matches!(command(session, text).await, IntentOutcome::Applied { .. }),
        "{text}"
    );
}

async fn add(session: &mut Attachment, endpoint: &MockServer) {
    login_command(
        session,
        &format!("/provider add work openai {}", endpoint.uri()),
        endpoint,
    )
    .await;
}

async fn login_command(session: &mut Attachment, text: &str, endpoint: &MockServer) {
    let intent = IntentId::mint();
    session
        .handle
        .submit(intent.clone(), Input::text(text, Origin::surface("test")));
    let dialog = tokio::time::timeout(LIMIT, async {
        loop {
            let frame = session.events.next().await.unwrap();
            session.snapshot.apply(&frame);
            if let Event::InteractionOpened { interaction } = frame.event {
                break interaction;
            }
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        dialog.kind,
        InteractionKind::Login {
            flow: LoginFlow::Paste,
            ..
        }
    ));
    assert!(
        endpoint.received_requests().await.unwrap().is_empty(),
        "no probe before paste"
    );
    session.handle.answer(
        IntentId::mint(),
        dialog.id,
        Answer::Text { text: KEY.into() },
        Activation::Programmatic,
    );
    assert!(matches!(
        ack_for(session, &intent).await,
        IntentOutcome::Applied { .. }
    ));
}

async fn answer(session: &mut Attachment, expected: &str) {
    session.handle.submit(
        IntentId::mint(),
        Input::text("hello", Origin::surface("test")),
    );
    let frames = until_completed(session).await;
    let text = serde_json::to_string(&frames).unwrap();
    assert!(text.contains(expected), "{text}");
    assert!(!text.contains(KEY));
}

async fn auth_required(session: &mut Attachment, reason: &str) {
    session.handle.submit(
        IntentId::mint(),
        Input::text("hello", Origin::surface("test")),
    );
    let frames = until_completed(session).await;
    let text = serde_json::to_string(&frames).unwrap();
    assert!(text.contains("AUTH_REQUIRED"), "{text}");
    assert!(text.contains(reason), "{text}");
}

async fn meta(kernel: &impl HostApi) -> serde_json::Value {
    kernel
        .catalog(CatalogKind::Providers)
        .await
        .unwrap()
        .entries
        .into_iter()
        .find(|e| e.id == "work")
        .unwrap()
        .meta
}

#[tokio::test(flavor = "multi_thread")]
async fn add_readdress_and_remove_without_restarting_or_rebinding_sessions() {
    let first = endpoint("First").await;
    let second = endpoint("Second").await;
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".bingo")).unwrap();
    std::fs::write(
        home.path().join(".bingo/settings.toml"),
        "# keep this comment\n[update]\ncheck = false\n",
    )
    .unwrap();
    let mut server = Server::spawn_at(home, IDLE, &[]);
    let kernel = ready(&mut server).await;
    let mut session = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    add(&mut session, &first).await;
    saved(&server, &first.uri());
    assert_eq!(meta(&kernel).await["reach"]["kind"], "ok");
    assert_eq!(meta(&kernel).await["auth"]["from"], "store");
    applied(&mut session, "/model work/local-model").await;
    answer(&mut session, "First").await;
    login_command(
        &mut session,
        &format!("/provider set work url {}", second.uri()),
        &second,
    )
    .await;
    auth_required(&mut session, "changed origin").await;
    let mut newer = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    applied(&mut newer, "/model work/local-model").await;
    answer(&mut newer, "Second").await;
    applied(&mut newer, "/models refresh work").await;
    assert!(!matches!(
        command(&mut newer, "/provider remove openai").await,
        IntentOutcome::Applied { .. }
    ));
    applied(&mut newer, "/provider remove work").await;
    auth_required(&mut newer, "work was removed").await;
    assert!(
        !kernel
            .catalog(CatalogKind::Providers)
            .await
            .unwrap()
            .entries
            .iter()
            .any(|e| e.id == "work")
    );
    assert!(
        !kernel
            .catalog(CatalogKind::Models)
            .await
            .unwrap()
            .entries
            .iter()
            .any(|e| e.id.starts_with("work/"))
    );
    let settings = std::fs::read_to_string(server.cwd().join(".bingo/settings.toml")).unwrap();
    assert!(settings.contains("# keep this comment"));
    assert!(!settings.contains(KEY));
    let auth = std::fs::read_to_string(server.cwd().join(".bingo/data/auth.json")).unwrap();
    assert!(!auth.contains(KEY));
    kernel.shutdown().await.unwrap();
    assert!(
        tokio::time::timeout(LIMIT, server.child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    no_secret(&server.sessions_dir());
}

fn no_secret(path: &std::path::Path) {
    for entry in std::fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            no_secret(&path);
        } else {
            assert!(!String::from_utf8_lossy(&std::fs::read(path).unwrap()).contains(KEY));
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn failed_reach_is_redacted_and_readdressing_repairs_it() {
    let bad = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(401).set_body_json(
            json!({"error":{"message":format!("Authorization: {KEY}\nhttps://host?key={KEY}")}}),
        ))
        .mount(&bad)
        .await;
    let good = endpoint("Fixed").await;
    let mut server = Server::spawn(IDLE);
    let kernel = ready(&mut server).await;
    let mut session = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    add(&mut session, &bad).await;
    let failed = meta(&kernel).await;
    assert_eq!(failed["reach"]["kind"], "down");
    assert!(!failed.to_string().contains(KEY));
    assert!(
        std::fs::read_to_string(server.cwd().join(".bingo/settings.toml"))
            .unwrap()
            .contains("work")
    );
    login_command(
        &mut session,
        &format!("/provider set work url {}", good.uri()),
        &good,
    )
    .await;
    assert_eq!(meta(&kernel).await["reach"]["kind"], "ok");
    kernel.shutdown().await.unwrap();
    assert!(
        tokio::time::timeout(LIMIT, server.child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    no_secret(&server.sessions_dir());
}

fn saved(server: &Server, url: &str) {
    let path = server.cwd().join(".bingo/settings.toml");
    let document = bingo_core::settings::read_document(&path).unwrap();
    assert_eq!(document["openai"]["instances"]["work"]["baseUrl"], url);
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains("# keep this comment"));
    assert!(!text.contains(KEY));
    let auth: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(server.cwd().join(".bingo/data/auth.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(auth["work"]["key"], KEY);
}

#[tokio::test(flavor = "multi_thread")]
async fn adding_refuses_foreign_credentials_and_invalid_secret_arguments() {
    let endpoint = endpoint("unused").await;
    let home = tempfile::tempdir().unwrap();
    let data = home.path().join(".bingo/data");
    std::fs::create_dir_all(&data).unwrap();
    let original = json!({"occupied":{"type":"api", "key":KEY}, "channels.telegram":{"type":"api", "key":KEY}}).to_string();
    std::fs::write(data.join("auth.json"), &original).unwrap();
    let mut server = Server::spawn_at(home, IDLE, &[]);
    let kernel = ready(&mut server).await;
    let mut session = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    for name in ["occupied", "channels.telegram", "service:key"] {
        let outcome = command(
            &mut session,
            &format!("/provider add {name} openai {}", endpoint.uri()),
        )
        .await;
        assert!(!matches!(outcome, IntentOutcome::Applied { .. }));
        if name == "occupied" {
            assert!(
                serde_json::to_string(&outcome)
                    .unwrap()
                    .contains("remove it first")
            );
        }
    }
    for text in [
        format!("/provider add work openai http://u:{KEY}@localhost"),
        format!("/provider add work openai {} {KEY}", endpoint.uri()),
    ] {
        assert!(!matches!(
            command(&mut session, &text).await,
            IntentOutcome::Applied { .. }
        ));
    }
    assert!(endpoint.received_requests().await.unwrap().is_empty());
    assert_eq!(
        std::fs::read_to_string(data.join("auth.json")).unwrap(),
        original
    );
    kernel.shutdown().await.unwrap();
    assert!(
        tokio::time::timeout(LIMIT, server.child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    no_secret(&server.sessions_dir());
}
