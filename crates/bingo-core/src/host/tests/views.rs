//! A plugin's live state through a real host (ADR-0013 §2): a signal is on
//! the attachment's stream and folded into the state, and nowhere in the
//! journal a client replays.

use super::*;

#[tokio::test]
async fn a_signal_is_live_and_never_journaled() {
    let store = Arc::new(crate::journal::MemoryStore::new());
    let host = host_on(
        store,
        ScriptedProvider::new(vec![
            Script::Events(text("one")),
            Script::Events(text("two")),
        ]),
    )
    .await;
    let mut a = host
        .open(
            SessionSelector::Create {
                spec: spec("/work"),
            },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    one_turn(&mut a, "first").await;
    let id = a.session.clone();

    host.signal(&id, "bingo.demo.ui", "progress", json!({"value": 3}))
        .await
        .expect("a live session takes a signal");
    let seen = a.events.next().await.expect("the stream is open");
    assert!(
        matches!(&seen.event, Event::Signal { plugin, kind, payload, .. }
            if plugin == "bingo.demo.ui" && kind == "progress" && payload == &json!({"value": 3})),
        "the signal is the next live frame: {seen:?}"
    );
    let mut folded = a.snapshot.clone();
    assert_eq!(folded.apply(&seen), Applied::Signal);
    assert_eq!(
        folded.signals["bingo.demo.ui"]["progress"],
        json!({"value": 3})
    );

    // A later durable frame bounds the replay; the signal is not in it.
    let end = one_turn(&mut a, "second").await;
    let mut replay = a.handle.events_since(Seq::ZERO).await.unwrap();
    while let Some(frame) = replay.next().await {
        assert!(
            !matches!(frame.event, Event::Signal { .. }),
            "a signal was journaled at {:?}",
            frame.seq
        );
        if frame.seq >= end {
            break;
        }
    }
}

#[tokio::test]
async fn generation_owned_signals_survive_older_cleanup_and_attachment() {
    let home = tempfile::tempdir().unwrap();
    let host = Host::build(vec![], HostConfig::new(env_in(home.path())))
        .await
        .unwrap();
    let mut a = host
        .open(
            SessionSelector::Create {
                spec: SessionSpec {
                    driver: Driver::Log,
                    cwd: home.path().into(),
                    ..Default::default()
                },
            },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    let old = ItemId::mint();
    let new = ItemId::mint();
    for (generation, payload) in [
        (&old, json!("old")),
        (&new, json!("new")),
        (&old, Value::Null),
    ] {
        bingo_sdk::generation::scope(
            "fixture",
            generation,
            host.plugin_host
                .signal(&a.session, "fixture", "pane", payload),
        )
        .await
        .unwrap();
    }
    let mut state = a.snapshot.clone();
    let mut signals = 0;
    while signals < 3 {
        let frame = a.events.next().await.unwrap();
        signals += usize::from(matches!(frame.event, Event::Signal { .. }));
        state.apply(&frame);
    }
    assert_eq!(state.signals["fixture"]["pane"], json!("new"));
    assert_eq!(state.signals["fixture"]["pane"].generation(), Some(&new));
    let a = host
        .open(
            SessionSelector::ById { id: a.session },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(
        a.snapshot.signals["fixture"]["pane"].generation(),
        Some(&new)
    );
    let replacement = ItemId::mint();
    host.plugin_host
        .transfer_signals(&a.session, "fixture", &old, &replacement)
        .await
        .unwrap();
    assert_eq!(
        signal_snapshot(&host, &a.session).await.signals["fixture"]["pane"].generation(),
        Some(&new)
    );
    host.plugin_host
        .transfer_signals(&a.session, "fixture", &new, &replacement)
        .await
        .unwrap();
    bingo_sdk::generation::scope(
        "fixture",
        &new,
        host.plugin_host
            .signal(&a.session, "fixture", "pane", Value::Null),
    )
    .await
    .unwrap();
    assert_eq!(
        signal_snapshot(&host, &a.session).await.signals["fixture"]["pane"].generation(),
        Some(&replacement)
    );
    bingo_sdk::generation::scope(
        "fixture",
        &replacement,
        host.plugin_host
            .signal(&a.session, "fixture", "pane", Value::Null),
    )
    .await
    .unwrap();
    assert!(signal_snapshot(&host, &a.session).await.signals.is_empty());
    host.shutdown().await;
}

async fn signal_snapshot(host: &Host, id: &SessionId) -> SessionState {
    host.open(
        SessionSelector::ById { id: id.clone() },
        who(),
        OpenOptions::default(),
    )
    .await
    .unwrap()
    .snapshot
}
