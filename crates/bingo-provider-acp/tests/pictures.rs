//! The fake agent records the provider's actual prompt, including picture order.
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod harness;
#[path = "harness/prompt_host.rs"]
mod prompt_host;

use bingo_provider_acp::{agent::AcpAgent, session::Sessions};
use bingo_sdk::{Agent, ContentPart, Env, Image, Message, ModelRequest, Role, SessionId};
use futures::StreamExt;
use harness::{Fake, LIMIT, agent_binary};
use serde_json::{Value, json};

fn request(parts: Vec<ContentPart>) -> ModelRequest {
    let mut user = Message::text(Role::User, "");
    user.parts = parts;
    ModelRequest {
        model: "agent".into(),
        max_tokens: 1,
        system: Vec::new(),
        messages: vec![user],
        tools: Vec::new(),
        reasoning: None,
        session: Some(SessionId::from_raw("ses_picture")),
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
    let adapter =
        serde_json::from_value(json!({ "command": agent_binary(), "env": fake.env() })).unwrap();
    let provider = AcpAgent::new("scripted".into(), adapter, sessions.clone());
    let image = Image::from_bytes("image/png", b"picture bytes preserved on the wire").unwrap();
    let parts = vec![
        ContentPart::text("before"),
        ContentPart::Image(image.clone()),
        ContentPart::text("between"),
        ContentPart::Image(image.clone()),
        ContentPart::text("after"),
    ];
    tokio::time::timeout(LIMIT, async {
        let mut stream = provider.run(harness::turn(request(parts))).await.unwrap();
        while let Some(event) = stream.next().await {
            event.unwrap();
        }
    })
    .await
    .expect("the fake agent finishes");
    let mut heard = fake.first("session/prompt").unwrap()["prompt"].clone();
    sessions.close().await;
    let first = heard[0]["text"].as_str().unwrap();
    assert!(
        first.starts_with("You have bingo's own tools over MCP") && first.ends_with("\n\nbefore"),
        "the opening preamble precedes the first user part: {first}"
    );
    heard[0]["text"] = json!("before");
    (heard, image)
}

#[tokio::test]
async fn an_image_capable_acp_agent_receives_each_picture_in_order() {
    let (prompt, image) = prompt(true).await;
    assert_eq!(
        prompt,
        json!([
            {"type":"text", "text":"before"},
            {"type":"image", "mimeType":image.media_type, "data":image.data},
            {"type":"text", "text":"between"},
            {"type":"image", "mimeType":image.media_type, "data":image.data},
            {"type":"text", "text":"after"}
        ])
    );
}

#[tokio::test]
async fn an_acp_agent_without_images_receives_one_note_per_picture() {
    let (prompt, image) = prompt(false).await;
    let note = format!(
        "[picture not sent: image/png {}; this agent does not take pictures]",
        bingo_sdk::bytes::words(image.decoded_len())
    );
    assert_eq!(
        prompt,
        json!([
            {"type":"text", "text":"before"}, {"type":"text", "text":note},
            {"type":"text", "text":"between"}, {"type":"text", "text":note},
            {"type":"text", "text":"after"}
        ])
    );
}

/// The bridge must consume AgentTurn.tools, not reconstruct a catalogue offer.
#[tokio::test]
async fn the_bridge_offers_exactly_the_turns_resolved_tools() {
    let fake = Fake::new(json!({"turns":[{"updates":[],"stopReason":"end_turn"}]}));
    let sessions = Sessions::new(Env::rooted(fake.cwd()));
    sessions.set_host(prompt_host::host(fake.cwd())).await;
    let adapter = serde_json::from_value(
        json!({"command":agent_binary(),"env":fake.env(),"tools":["Resolved","Absent"]}),
    )
    .unwrap();
    let agent = AcpAgent::new("scripted".into(), adapter, sessions.clone());
    let mut turn = harness::turn(request(vec![ContentPart::text("list")]));
    turn.tools = vec![bingo_sdk::ToolSpec {
        look: None,
        name: "Resolved".into(),
        description: "the turn resolved this".into(),
        input_schema: json!({"type":"object"}),
        meta: Default::default(),
    }];
    tokio::time::timeout(LIMIT, async {
        let mut stream = agent.run(turn).await.unwrap();
        while let Some(event) = stream.next().await {
            event.unwrap();
        }
    })
    .await
    .unwrap();
    use bingo_provider_acp::bridge::{ADDRESS_VAR, Address, TOKEN_VAR, handshake, socket};
    use rmcp::ServiceExt;
    let opened = fake.first("session/new").unwrap();
    let env = opened["mcpServers"][0]["env"].as_array().unwrap();
    let value = |name| {
        env.iter().find(|v| v["name"] == name).unwrap()["value"]
            .as_str()
            .unwrap()
    };
    let address = Address::from_raw(value(ADDRESS_VAR));
    tokio::time::timeout(LIMIT, async {
        let mut wire = socket::dial(&address).await.unwrap();
        handshake::write(&mut wire, value(TOKEN_VAR)).await.unwrap();
        let client = ().serve(wire).await.unwrap();
        let listed = client.list_tools(None).await.unwrap();
        assert_eq!(listed.tools.len(), 1, "{listed:?}");
        assert_eq!(listed.tools[0].name, "Resolved");
        assert_eq!(
            listed.tools[0].description.as_deref(),
            Some("the turn resolved this")
        );
        client.cancel().await.unwrap();
    })
    .await
    .unwrap();
    sessions.close().await;
}
