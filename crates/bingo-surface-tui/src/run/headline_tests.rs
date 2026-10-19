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

#[test]
fn image_refusal_is_checked_once_at_the_run_boundary() {
    let mut state = crate::test_support::state();
    state.summary.model = Some("text-only".into());
    state.summary.provider = Some("fake".into());
    let mut run = super::tests::idle_in(
        state,
        Arc::new(crate::doubles::TestSession::default()),
        std::time::Instant::now(),
    );
    run.ui.catalogs.values.insert(
        "models".into(),
        vec![bingo_sdk::CatalogEntry {
            id: "fake/text-only".into(),
            label: "text-only".into(),
            meta: serde_json::json!({"images":false}),
        }],
    );
    run.clipboard_image = Arc::new(|| panic!("a refused model never reads the clipboard"));
    run.terminal_event(crossterm::event::Event::Key(crate::test_support::ctrl('v')));
    assert!(run.ui.composer.is_empty());
    assert!(
        run.ui
            .notice()
            .unwrap()
            .plain()
            .unwrap()
            .text
            .contains("alt+m switches model")
    );
}
