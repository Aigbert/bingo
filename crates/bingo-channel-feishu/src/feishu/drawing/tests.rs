use super::*;
use crate::feishu::tests::{bodies, feishu, ok};
use bingo_sdk::{
    SessionId, View,
    render::{Component, RenderRequest, SurfaceName, Viewport},
};
use wiremock::MockServer;

fn answer() -> Drawing {
    Drawing {
        session: SessionId::from_raw("s"),
        cwd: "/work".into(),
        request: RenderRequest {
            component: Component::AssistantMessage,
            request_id: "answer".into(),
            surface: SurfaceName::Chat,
            props: json!({"text":"engine","isFirstOfReply":true}),
            viewport: Viewport {
                columns: 0,
                rows: 0,
                place: None,
            },
        },
        view: View::text("Site answer"),
        footer: "model · out 10".into(),
    }
}
#[tokio::test]
async fn replacing_a_finished_stream_keeps_renewal_footer_and_monotonic_sequences() {
    let server = MockServer::start().await;
    let feishu = feishu(&server).await;
    ok(&server, "POST", CARDS, json!({"card_id":"card"})).await;
    ok(&server, "POST", MESSAGES, json!({"message_id":"message"})).await;
    let element = format!("{CARDS}/card/elements/answer/content");
    let settings = format!("{CARDS}/card/settings");
    let replace = format!("{CARDS}/card");
    ok(&server, "PUT", &element, json!({})).await;
    ok(&server, "PATCH", &settings, json!({})).await;
    ok(&server, "PUT", &replace, json!({})).await;
    let to = Conversation::direct("chat");
    let at = feishu.send(&to, "", Mode::Stream).await.unwrap();
    locked(&feishu.streaming_since).insert("card".into(), Instant::now() - STREAMING_RENEWED);
    assert_eq!(
        feishu.draw(&to, None, Some(&at), &answer()).await.unwrap(),
        at
    );
    let toggles = bodies(&server, &settings).await;
    assert_eq!(
        toggles[0]["settings"],
        json!({"config":{"streaming_mode":true}}).to_string()
    );
    assert_eq!(
        toggles[1]["settings"],
        json!({"config":{"streaming_mode":false}}).to_string()
    );
    let replaced = &bodies(&server, &replace).await[0];
    assert_eq!(replaced["sequence"], 4);
    let card: Value = serde_json::from_str(replaced["card"]["data"].as_str().unwrap()).unwrap();
    assert_eq!(
        card["body"]["elements"][1]["content"],
        "---\nmodel · out 10"
    );
    feishu.draw(&to, None, Some(&at), &answer()).await.unwrap();
    assert_eq!(
        bodies(&server, &element).await.len(),
        1,
        "redraw does not reopen a finished stream"
    );
    assert_eq!(bodies(&server, &replace).await[1]["sequence"], 5);
}
#[tokio::test]
async fn an_image_is_uploaded_and_the_card_uses_only_the_returned_key() {
    let server = MockServer::start().await;
    let feishu = feishu(&server).await;
    ok(&server, "POST", IMAGES, json!({"image_key":"uploaded"})).await;
    ok(&server, "POST", CARDS, json!({"card_id":"card"})).await;
    ok(&server, "POST", MESSAGES, json!({"message_id":"message"})).await;
    let dir = tempfile::tempdir().unwrap();
    use base64::Engine;
    let png = base64::engine::general_purpose::STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAIAAAD91JpzAAAAEElEQVR42mO4WR4ORAwQCgAwWgadFBKCuwAAAABJRU5ErkJggg==").unwrap();
    std::fs::write(dir.path().join("picture.png"), png).unwrap();
    let mut drawing = answer();
    drawing.cwd = dir.path().into();
    drawing.view =
        serde_json::from_value(json!({"kind":"Image","source":"picture.png","alt":"Picture"}))
            .unwrap();
    feishu
        .draw(&Conversation::direct("chat"), None, None, &drawing)
        .await
        .unwrap();
    let sent: Value =
        serde_json::from_str(bodies(&server, CARDS).await[0]["data"].as_str().unwrap()).unwrap();
    assert_eq!(sent["body"]["elements"][0]["img_key"], "uploaded");
}
