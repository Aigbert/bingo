use super::*;

/// A home whose settings put the fake model on `high` and declare that it
/// reasons at all: without the declaration every level is filtered out of
/// every request (ADR-0004) and the view would say `null` three times.
fn thinking_home() -> (tempfile::TempDir, String) {
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join("settings.json");
    std::fs::write(
        &path,
        r#"{"thinking": "high", "models": {"fake/fake-1": {"reasoning": true}}}"#,
    )
    .unwrap();
    let settings = path.display().to_string();
    (home, settings)
}

/// The level the config view says this session's next turn will ask for: off
/// the snapshot when the cut already carried it, else the first
/// `ConfigChanged` after it.
async fn thinking_of(attachment: &mut Attachment) -> serde_json::Value {
    if !attachment.snapshot.config.kernel.is_null() {
        return attachment.snapshot.config.kernel["thinking"].clone();
    }
    let folded = async {
        while let Some(frame) = attachment.events.next().await {
            attachment.snapshot.apply(&frame);
            if matches!(frame.event, Event::ConfigChanged { .. }) {
                return attachment.snapshot.config.kernel["thinking"].clone();
            }
        }
        panic!("the stream ended before the config view said anything");
    };
    tokio::time::timeout(LIMIT, folded)
        .await
        .expect("the config view is published at start")
}

/// ADR-0047 §1 on the wire: `session/open` takes the level in the spec, with
/// no method change, and every client learns what the next turn will ask for
/// from the config view it already folds. The three answers are told apart —
/// absent inherits the settings, `null` is off, a word is that level.
#[tokio::test(flavor = "multi_thread")]
async fn session_open_takes_a_thinking_level_and_the_config_view_says_it() {
    let (home, settings) = thinking_home();
    let mut server = Server::spawn_at(home, TEXT_TURN, &["--settings", &settings]);
    let kernel = ready(&mut server).await;

    let asked = [
        (None, serde_json::json!("high")),
        (Some(None), serde_json::Value::Null),
        (Some(Some(Effort::Low)), serde_json::json!("low")),
    ];
    for (thinking, effective) in asked {
        let selector = SessionSelector::Create {
            spec: SessionSpec {
                cwd: server.cwd(),
                thinking,
                ..SessionSpec::default()
            },
        };
        let mut attachment = kernel
            .open(selector, who(), OpenOptions::default())
            .await
            .unwrap();
        assert_eq!(
            thinking_of(&mut attachment).await,
            effective,
            "a spec that says {thinking:?} runs at {effective}"
        );
    }
    kernel.shutdown().await.unwrap();
}
