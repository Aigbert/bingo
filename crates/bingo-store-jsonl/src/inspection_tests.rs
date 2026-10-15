use super::*;
use crate::tests::*;
use bingo_sdk::{ContentPart, Item, ItemBody, ItemId, ItemStatus, Origin, SessionState, Voice};

#[tokio::test]
async fn stored_read_and_title_preserve_activity_and_release_the_claim() {
    let (_root, store) = root_with();
    let before = summary();
    store.create(&before).await.unwrap();
    store
        .append(
            &before.id,
            &frame(
                1,
                Event::SessionUpdated {
                    summary: before.clone(),
                },
            ),
        )
        .await
        .unwrap();
    let path = layout::journal(&layout::session_dir(store.root(), &before.id));
    let bytes = std::fs::read(&path).unwrap();
    let before = store.summary(&before.id).await.unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    store.rename(&before.id, "changed").await.unwrap();
    let after = store.summary(&before.id).await.unwrap();
    assert_eq!(after.title.as_deref(), Some("changed"));
    assert_eq!(after.updated_at, before.updated_at);
    assert_eq!(
        store.list(&SessionFilter::default()).await.unwrap()[0].updated_at,
        before.updated_at
    );
    assert_eq!(store.replay(&before.id, Seq::ZERO).await.unwrap().len(), 2);
    let other = JsonlStore::new(store.root());
    other.acquire(&before.id).await.unwrap();
    assert_eq!(
        store.rename(&before.id, "stolen").await.unwrap_err().code,
        ErrorCode::SessionLocked
    );
    assert_eq!(store.replay(&before.id, Seq::ZERO).await.unwrap().len(), 2);
    other.release(&before.id).await.unwrap();
}

fn user(seq: u64, text: &str, voice: Option<Voice>) -> Frame {
    let mut origin = Origin::surface("tui");
    origin.voice = voice;
    message(
        seq,
        ItemBody::User {
            parts: vec![ContentPart::text(text)],
            origin,
        },
    )
}

fn message(seq: u64, body: ItemBody) -> Frame {
    frame(
        seq,
        Event::ItemCompleted {
            item: Item {
                id: ItemId::from_raw(format!("itm_{seq}")),
                turn: None,
                round: 0,
                status: ItemStatus::Completed,
                started_at: stamp(),
                completed_at: Some(stamp()),
                intent: None,
                meta: Default::default(),
                body,
            },
        },
    )
}

/// The live fold, the appended file and the rebuilt file are three
/// derivations of one fact; they must never disagree.
#[tokio::test]
async fn stored_and_live_first_asks_agree() {
    let (_root, store) = root_with();
    let long = format!("  why {}  \nsecond line", "x".repeat(300));
    let frames = [
        frame(1, Event::SessionUpdated { summary: summary() }),
        user(2, "a wake note", Some(Voice::Note)),
        message(3, ItemBody::Assistant { text: "hi".into() }),
        user(4, &long, None),
        user(5, "a later ask", None),
    ];
    store.create(&summary()).await.unwrap();
    let mut live = SessionState::new(summary());
    for frame in &frames {
        store.append(&session(), frame).await.unwrap();
        live.apply(frame);
    }
    let dir = layout::session_dir(store.root(), &session());
    let appended = summary::read(&dir).unwrap().history.first_ask;
    std::fs::remove_file(layout::summary(&dir)).unwrap();
    let rebuilt = summary::of(&dir).unwrap().unwrap().history.first_ask;
    let expected: String = format!("why {}", "x".repeat(300))
        .chars()
        .take(200)
        .collect();
    assert_eq!(live.summary.history.first_ask.as_deref(), Some(&*expected));
    assert_eq!(appended, live.summary.history.first_ask);
    assert_eq!(rebuilt, live.summary.history.first_ask);
}
