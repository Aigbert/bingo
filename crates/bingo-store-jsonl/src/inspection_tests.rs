use super::*;
use crate::tests::*;

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
