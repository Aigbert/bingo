//! Where the context stands before a round has measured it: the reading a
//! session publishes when it opens and when its model changes.
use super::commands::Client;
use super::*;

async fn setup(home: &std::path::Path, model: &str) -> Arc<Host> {
    serving(home, model, vec![Script::Events(text("hello"))], None).await
}

async fn serving(
    home: &std::path::Path,
    model: &str,
    scripts: Vec<Script>,
    tool: Option<Arc<dyn Tool>>,
) -> Arc<Host> {
    let provider = ScriptedProvider::new(scripts);
    provider.serves(&["large", "small", "guess"]);
    let mut plugins = vec![TestPlugin::boxed(
        &PROVIDER,
        vec![Contribution::Provider(provider)],
    )];
    if let Some(tool) = tool {
        plugins.push(TestPlugin::boxed(&TOOLS, vec![Contribution::Tool(tool)]));
    }
    let config = HostConfig::new(env_in(home)).with_layer("test", json!({
        "provider":"scripted", "model":model, "maxTokens":1000,
        "models":{"scripted/large":{"contextWindow":200000}, "scripted/small":{"contextWindow":8000}}
    }));
    Host::build(plugins, config).await.unwrap()
}

fn resting(window: u64, trigger: u64) -> ContextUsage {
    ContextUsage {
        parts: None,
        used: 0,
        window,
        trigger,
    }
}

fn changes(frames: &[Frame]) -> Vec<Option<ContextUsage>> {
    frames
        .iter()
        .filter_map(|frame| match frame.event {
            Event::ContextChanged { context } => Some(context),
            _ => None,
        })
        .collect()
}

/// The window is on the ruler before the first request measures anything,
/// and it is the same effective window that request will be measured in.
#[tokio::test]
async fn a_known_window_is_published_before_the_first_round() {
    let home = tempfile::tempdir().unwrap();
    let host = setup(home.path(), "large").await;
    let client = Client::open(&host).await;
    assert!(client.state.turn.is_none() && client.state.last_turn.is_none());
    assert_eq!(client.state.context, Some(resting(199_000, 179_100)));
    host.shutdown().await;
}

/// The fallback guess is not a window anybody stated: nothing is published
/// and a client keeps saying it does not know.
#[tokio::test]
async fn a_guessed_window_publishes_nothing_before_the_first_round() {
    let home = tempfile::tempdir().unwrap();
    let host = setup(home.path(), "guess").await;
    let client = Client::open(&host).await;
    assert_eq!(client.state.context, None);
    host.shutdown().await;
}

#[tokio::test]
async fn a_model_change_restates_the_window_and_a_guess_clears_it() {
    let home = tempfile::tempdir().unwrap();
    let host = setup(home.path(), "large").await;
    let mut client = Client::open(&host).await;
    let (_, frames) = client.ack("/model small").await;
    assert_eq!(changes(&frames), vec![Some(resting(7_000, 6_300))]);
    let (_, frames) = client.ack("/think high").await;
    assert_eq!(changes(&frames), vec![], "the same model restates nothing");
    let (_, frames) = client.ack("/model guess").await;
    assert_eq!(changes(&frames), vec![None]);
    assert_eq!(client.state.context, None);
    host.shutdown().await;
}

/// A read-only tool that holds its round open until the test lets it go.
struct Held(Arc<tokio::sync::Notify>);

#[async_trait]
impl Tool for Held {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            look: None,
            name: "Echo".into(),
            description: "held".into(),
            input_schema: json!({"type": "object"}),
            meta: Default::default(),
        }
    }
    fn traits(&self, _: &Value) -> ToolTraits {
        ToolTraits::read_only()
    }
    async fn call(&self, _: Value, _: &ToolContext) -> Result<ToolOutput, ToolError> {
        self.0.notified().await;
        Ok(ToolOutput::text("released"))
    }
}

/// A switch while a turn runs is stated at once, the running turn's next
/// round reads the old model's window over it, and the turn's end states the
/// new model's again.
#[tokio::test]
async fn a_switch_in_a_running_turn_is_restated_when_the_turn_ends() {
    let home = tempfile::tempdir().unwrap();
    let gate = Arc::new(tokio::sync::Notify::new());
    let scripts = vec![
        Script::Events(tool_call("Echo", json!({}))),
        Script::Events(text("done")),
    ];
    let host = serving(
        home.path(),
        "large",
        scripts,
        Some(Arc::new(Held(gate.clone()))),
    )
    .await;
    let mut client = Client::open(&host).await;
    client.ack("go").await;
    while !matches!(
        client.events.next().await.map(|frame| {
            client.state.apply(&frame);
            frame.event
        }),
        Some(Event::TurnUsage { .. })
    ) {}
    let (_, frames) = client.ack("/model small").await;
    assert_eq!(changes(&frames).len(), 1, "the switch is stated at once");
    gate.notify_one();
    let frames = client.until_turn_completed().await;
    let round = frames.iter().rev().find_map(|frame| match frame.event {
        Event::TurnUsage { context, .. } => Some(context),
        _ => None,
    });
    assert_eq!(
        round.unwrap().window,
        199_000,
        "the turn ran on the old model"
    );
    let restated = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        loop {
            let frame = client.events.next().await.expect("the session stays open");
            client.state.apply(&frame);
            if let Event::ContextChanged { context } = frame.event {
                return context;
            }
        }
    })
    .await
    .unwrap();
    let restated = restated.unwrap();
    assert_eq!((restated.window, restated.trigger), (7_000, 6_300));
    assert_eq!(client.state.context, Some(restated));
    host.shutdown().await;
}

/// `/status` reads the breakdown of the last measure; a switch that leaves
/// the request its size keeps it.
#[tokio::test]
async fn a_switch_keeps_the_breakdown_of_a_request_of_the_same_size() {
    let home = tempfile::tempdir().unwrap();
    let mut events = text("hello");
    if let Ok(ModelEvent::Finish { usage, .. }) = events.last_mut().unwrap() {
        usage.input_tokens = 5_000;
    }
    let host = serving(home.path(), "large", vec![Script::Events(events)], None).await;
    let mut client = Client::open(&host).await;
    client.ack("hello").await;
    client.until_turn_completed().await;
    let measured = client.state.context.unwrap();
    assert!(measured.parts.is_some());
    let (_, frames) = client.ack("/model small").await;
    let switched = changes(&frames)[0].unwrap();
    assert_eq!((switched.used, switched.window), (measured.used, 7_000));
    assert_eq!(switched.parts, measured.parts);
    host.shutdown().await;
}
