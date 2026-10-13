//! The fake agent records the provider's actual prompt, including picture order.
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod harness;
#[path = "harness/prompt_host.rs"]
mod prompt_host;

use bingo_provider_acp::{provider::AcpProvider, session::Sessions};
use bingo_sdk::{CancellationToken, ContentPart, Env, Image, Message, ModelRequest, Provider, Role, SessionId};
use futures::StreamExt;
use harness::{Fake, LIMIT, agent_binary};
use serde_json::{Value, json};

fn request(parts: Vec<ContentPart>) -> ModelRequest {
    let mut user = Message::text(Role::User, "");
    user.parts = parts;
    ModelRequest {
        model: "agent".into(), max_tokens: 1, system: Vec::new(),
        messages: vec![user],
        tools: Vec::new(), reasoning: None, session: Some(SessionId::from_raw("ses_picture")),
        provider_options: Default::default(),
    }
}

async fn prompt(images: bool) -> (Value, Image) {
    let fake = Fake::new(json!({
        "capabilities": { "image": images },
        "turns": [{ "updates": [], "stopReason": "end_turn" }]
    }));
    let sessions = Sessions::new(Env::rooted(fake.cwd()));
    sessions.set_host(prompt_host::host(fake.cwd())).await;
    let adapter = serde_json::from_value(json!({ "command": agent_binary(), "env": fake.env() })).unwrap();
    let provider = AcpProvider::new("scripted".into(), adapter, sessions.clone());
    let image = Image::from_bytes("image/png", b"picture bytes preserved on the wire").unwrap();
    let parts = vec![ContentPart::text("before"), ContentPart::Image(image.clone()),
        ContentPart::text("between"), ContentPart::Image(image.clone()), ContentPart::text("after")];
    tokio::time::timeout(LIMIT, async {
        let mut stream = provider.stream(request(parts), CancellationToken::new()).await.unwrap();
        while let Some(event) = stream.next().await { event.unwrap(); }
    }).await.expect("the fake agent finishes");
    let mut heard = fake.first("session/prompt").unwrap()["prompt"].clone();
    sessions.close().await;
    let first = heard[0]["text"].as_str().unwrap();
    assert!(first.starts_with("You have bingo's own tools over MCP") && first.ends_with("\n\nbefore"),
        "the opening preamble precedes the first user part: {first}");
    heard[0]["text"] = json!("before");
    (heard, image)
}

#[tokio::test]
async fn an_image_capable_acp_agent_receives_each_picture_in_order() {
    let (prompt, image) = prompt(true).await;
    assert_eq!(prompt, json!([
        {"type":"text", "text":"before"},
        {"type":"image", "mimeType":image.media_type, "data":image.data},
        {"type":"text", "text":"between"},
        {"type":"image", "mimeType":image.media_type, "data":image.data},
        {"type":"text", "text":"after"}
    ]));
}

#[tokio::test]
async fn an_acp_agent_without_images_receives_one_note_per_picture() {
    let (prompt, image) = prompt(false).await;
    let note = format!("[picture not sent: image/png {}; this agent does not take pictures]",
        bingo_sdk::bytes::words(image.decoded_len()));
    assert_eq!(prompt, json!([
        {"type":"text", "text":"before"}, {"type":"text", "text":note},
        {"type":"text", "text":"between"}, {"type":"text", "text":note},
        {"type":"text", "text":"after"}
    ]));
}
