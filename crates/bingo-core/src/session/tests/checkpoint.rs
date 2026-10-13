use super::*;

async fn cut_actor() -> Actor {
    let (mut actor, _) = storage::failing_actor().await;
    actor.open().await.unwrap().await;
    for n in 1..=4 {
        let mut item = actor.fresh(
            None,
            None,
            ItemBody::Assistant {
                text: n.to_string(),
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
