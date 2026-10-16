//! Where the context stands before a round has measured it: the reading a
//! session publishes when it opens and when its model changes.
use super::commands::Client;
use super::*;

async fn setup(home: &std::path::Path, model: &str) -> Arc<Host> {
    let provider = ScriptedProvider::new(vec![Script::Events(text("hello"))]);
    provider.serves(&["large", "small", "guess"]);
    let plugins = vec![TestPlugin::boxed(
        &PROVIDER,
        vec![Contribution::Provider(provider)],
    )];
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
