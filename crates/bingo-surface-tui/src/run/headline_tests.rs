//! A real Host keeps the headline off the fake provider's wire.
use bingo_sdk::{Event, Headline, HostApi, Input, IntentId, SessionSelector, SessionSpec, Tone};
use futures::StreamExt;
use std::sync::Arc;

#[tokio::test]
async fn the_fake_provider_reads_text_but_never_the_headline() {
    let home = tempfile::tempdir().unwrap();
    let provider = Arc::new(bingo_provider_fake::FakeProvider::demo());
    let host = bingo_core::Host::build(
        vec![Box::new(bingo_provider_fake::FakePlugin::new(
            provider.clone(),
        ))],
        bingo_core::HostConfig::new(bingo_sdk::Env::rooted(home.path())),
    )
    .await
    .unwrap();
    let mut attached = host
        .open(
            SessionSelector::Create {
                spec: SessionSpec {
                    cwd: home.path().into(),
                    ..Default::default()
                },
            },
            super::identity(),
            Default::default(),
        )
        .await
        .unwrap();
    attached.handle.submit(
        IntentId::mint(),
        Input::text("model-only-words", crate::origin()).with_headline(Headline {
            tone: Tone::Good,
            parts: vec!["never-in-a-provider-request".into()],
        }),
    );
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(frame) = attached.events.next().await {
            if matches!(frame.event, Event::TurnCompleted { .. }) {
                return;
            }
        }
        panic!("no turn completion");
    })
    .await
    .unwrap();
    let requests = serde_json::to_string(&provider.requests()).unwrap();
    assert!(requests.contains("model-only-words"));
    assert!(!requests.contains("never-in-a-provider-request"));
    host.shutdown().await;
}
