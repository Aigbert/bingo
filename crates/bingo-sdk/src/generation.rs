//! Resource ownership follows the loaded generation through asynchronous calls.
use crate::ItemId;
use std::future::Future;

tokio::task_local! { static CURRENT: (String, ItemId); }

pub fn current(plugin: &str) -> Option<ItemId> {
    CURRENT
        .try_with(|(owner, id)| (owner == plugin).then(|| id.clone()))
        .ok()
        .flatten()
}

pub async fn scope<T>(plugin: &str, id: &ItemId, future: impl Future<Output = T>) -> T {
    CURRENT.scope((plugin.into(), id.clone()), future).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn ownership_does_not_follow_calls_into_other_plugins() {
        let id = ItemId::mint();
        scope("one", &id, async {
            assert_eq!(current("one"), Some(id.clone()));
            assert_eq!(current("two"), None);
        })
        .await;
        assert_eq!(current("one"), None);
    }
}
