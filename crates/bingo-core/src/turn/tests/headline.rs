use super::*;

#[tokio::test]
async fn a_steer_absorbed_at_the_barrier_keeps_its_headline() {
    let provider = ScriptedProvider::new(vec![
        Script::Events(tool_call("Echo", json!({"v":1}))),
        Script::Events(text("ok")),
    ]);
    let cfg = config(provider, vec![Arc::new(EchoTool { read_only: true })]);
    let host = RecordingHost::new();
    let headline = Headline {
        tone: Tone::Good,
        parts: vec!["person's words".into()],
    };
    host.queue.lock().unwrap().push((
        IntentId::mint(),
        Input::text("steer", Origin::surface("test")).with_headline(headline.clone()),
    ));
    run(&cfg, &host, CancellationToken::new()).await;
    assert!(host.events().iter().any(|event| matches!(event, Event::ItemCompleted { item } if item.headline() == Some(headline.clone()))));
}
