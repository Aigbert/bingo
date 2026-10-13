use super::*;

async fn cut_actor() -> Actor {
    let (mut actor, _) = storage::failing_actor().await;
    actor.open().await.unwrap().await;
    for n in 1..=4 {
        let mut item = actor.fresh(
            None,
            None,
            ItemBody::Assistant {
                text: if n == 2 {
                    "x".repeat(16_384)
                } else {
                    n.to_string()
                },
            },
        );
        item.id = ItemId::from_raw(format!("i{n}"));
        actor
            .publish(Event::ItemCompleted { item }, None)
            .await
            .unwrap();
    }
    actor
        .publish(
            Event::Compacted {
                generation: 1,
                boundary: ItemId::from_raw("i3"),
                kept: vec![ItemId::from_raw("i1")],
                summary: ItemId::from_raw("i4"),
            },
            None,
        )
        .await
        .unwrap();
    actor.tracker.close();
    actor.tracker.wait().await;
    while let Ok(message) = actor.rx.try_recv() {
        actor.handle(message).await;
    }
    actor
}

#[tokio::test]
async fn checkpoint_ack_releases_only_unneeded_items_and_covered_frames() {
    let actor = cut_actor().await;
    assert!(actor.journal.is_empty());
    assert!(
        actor
            .state
            .transcript
            .get(&ItemId::from_raw("i2"))
            .is_none()
    );
    assert_eq!(
        actor
            .state
            .transcript
            .model_items()
            .map(|i| i.id.as_str())
            .collect::<Vec<_>>(),
        ["i1", "i4", "i3"]
    );
    assert_eq!(
        actor
            .state
            .transcript
            .items()
            .iter()
            .map(|i| i.id.as_str())
            .collect::<Vec<_>>(),
        ["i3", "i4"]
    );
}

#[tokio::test]
async fn checkpoint_catch_up_includes_the_cut_once_then_live_frames() {
    let mut actor = cut_actor().await;
    let expected = actor
        .store
        .as_ref()
        .unwrap()
        .replay(&actor.id, Seq::ZERO)
        .await
        .unwrap();
    let mut stream = actor.subscribe(Seq::ZERO);
    let id = actor
        .record(ItemBody::Assistant {
            text: "live".into(),
        })
        .await
        .unwrap();
    for frame in expected {
        assert_eq!(stream.next().await.unwrap(), frame);
    }
    let frame = stream.next().await.unwrap();
    assert!(matches!(frame.event, Event::ItemCompleted { item } if item.id == id));
    assert!(stream.next().now_or_never().is_none());
}

async fn settle(actor: &mut Actor) {
    actor.tracker.wait().await;
    while let Ok(message) = actor.rx.try_recv() {
        actor.handle(message).await;
    }
}

#[tokio::test]
async fn checkpoint_history_pages_join_the_client_window_without_duplicates() {
    let mut actor = cut_actor().await;
    let (snapshot, history) = actor.bounded_snapshot(1024 * 1024);
    let history = history.unwrap();
    assert_eq!(history.before, Some(ItemId::from_raw("i3")));
    assert!(history.has_more);
    let (tx, rx) = tokio::sync::oneshot::channel();
    actor.history_read(
        HistoryPage {
            before: history.before,
            generation: Some(1),
            limit: 0,
            max_bytes: Some(1024 * 1024),
        },
        tx,
    );
    settle(&mut actor).await;
    let page = rx.await.unwrap().unwrap();
    assert_eq!(
        page.items.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
        ["i1", "i2"]
    );
    assert!(page.next.is_none());
    let ids: Vec<_> = page
        .items
        .iter()
        .map(|i| i.id.as_str())
        .chain(snapshot.transcript.items().iter().map(|i| i.id.as_str()))
        .collect();
    assert_eq!(ids, ["i1", "i2", "i3", "i4"]);
    let (tx, rx) = tokio::sync::oneshot::channel();
    actor.pin_read(ItemId::from_raw("i2"), 1, tx);
    settle(&mut actor).await;
    assert_eq!(rx.await.unwrap().unwrap().id.as_str(), "i2");
    assert!(
        actor
            .state
            .transcript
            .get(&ItemId::from_raw("i2"))
            .is_none()
    );
}

#[tokio::test]
async fn checkpoint_unbounded_attachment_restores_full_history_at_its_cut() {
    let mut actor = cut_actor().await;
    let cut = actor.seq;
    let (tx, rx) = tokio::sync::oneshot::channel();
    actor.attach_read(None, tx);
    let new = actor
        .record(ItemBody::Assistant {
            text: "after cut".into(),
        })
        .await
        .unwrap();
    settle(&mut actor).await;
    let (snapshot, history, mut events) = rx.await.unwrap();
    assert_eq!(snapshot.seq, cut);
    assert!(history.is_none());
    assert_eq!(
        snapshot
            .transcript
            .items()
            .iter()
            .map(|i| i.id.as_str())
            .collect::<Vec<_>>(),
        ["i1", "i2", "i3", "i4"]
    );
    assert!(
        matches!(events.next().await.unwrap().event, Event::ItemCompleted { item } if item.id == new)
    );
    assert!(
        actor
            .state
            .transcript
            .get(&ItemId::from_raw("i2"))
            .is_none()
    );
}

#[tokio::test]
async fn checkpoint_storage_page_rechecks_generation_on_delivery() {
    let mut actor = cut_actor().await;
    let (tx, rx) = tokio::sync::oneshot::channel();
    actor.history_read(
        HistoryPage {
            before: Some(ItemId::from_raw("i3")),
            generation: Some(1),
            limit: 2,
            max_bytes: None,
        },
        tx,
    );
    actor
        .publish(
            Event::Rewound {
                generation: 2,
                to_turn: TurnId::from_raw("t"),
                dropped: vec![ItemId::from_raw("i4")],
                files_restored: vec![],
            },
            None,
        )
        .await
        .unwrap();
    settle(&mut actor).await;
    assert_eq!(
        rx.await.unwrap().unwrap_err().code,
        ErrorCode::StaleGeneration
    );
}

#[tokio::test]
async fn checkpoint_failure_retains_the_actor_window() {
    let (mut actor, _) = storage::failing_actor().await;
    actor.open().await.unwrap().await;
    let count = actor.journal.len();
    actor.checkpointed(actor.seq, Err(KernelError::new(ErrorCode::Storage, "full")));
    assert_eq!(actor.journal.len(), count);
    assert_eq!(actor.checkpoint_seq, Seq::ZERO);
}

#[tokio::test]
async fn checkpoint_oversized_history_item_is_pinned_from_storage() {
    let mut actor = cut_actor().await;
    let (tx, rx) = tokio::sync::oneshot::channel();
    actor.history_read(
        HistoryPage {
            before: Some(ItemId::from_raw("i3")),
            generation: Some(1),
            limit: 0,
            max_bytes: Some(1024),
        },
        tx,
    );
    settle(&mut actor).await;
    let page = rx.await.unwrap().unwrap();
    assert!(page.items.is_empty());
    assert_eq!(page.next, Some(ItemId::from_raw("i2")));
    let marker = page.oversized.unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel();
    actor.pin_read(marker.id.clone(), 1, tx);
    settle(&mut actor).await;
    let bytes = serde_json::to_vec(&rx.await.unwrap().unwrap()).unwrap();
    assert_eq!(bytes.len(), marker.total_bytes);
    assert_eq!(fnv1a64(&bytes), marker.checksum);
    let (tx, rx) = tokio::sync::oneshot::channel();
    actor.history_read(
        HistoryPage {
            before: Some(marker.id),
            generation: Some(1),
            limit: 0,
            max_bytes: Some(1024),
        },
        tx,
    );
    settle(&mut actor).await;
    let page = rx.await.unwrap().unwrap();
    assert_eq!(
        page.items.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
        ["i1"]
    );
    assert!(page.next.is_none());
}

#[tokio::test]
async fn checkpoint_io_does_not_block_actor_or_discard_newer_frames() {
    let (mut actor, store) = storage::failing_actor().await;
    actor.open().await.unwrap().await;
    let release = store.hold_checkpoints();
    let cut = actor.seq;
    actor.write_checkpoint();
    let id = actor
        .record(ItemBody::Assistant {
            text: "while checkpoint waits".into(),
        })
        .await
        .unwrap();
    assert_eq!(actor.checkpoint_seq, Seq::ZERO);
    assert!(actor.journal.iter().any(|frame| frame.seq <= cut));
    release.notify_one();
    actor.tracker.close();
    settle(&mut actor).await;
    assert_eq!(actor.checkpoint_seq, cut);
    assert_eq!(actor.journal.len(), 1);
    assert!(matches!(&actor.journal[0].event, Event::ItemCompleted { item } if item.id == id));
}
