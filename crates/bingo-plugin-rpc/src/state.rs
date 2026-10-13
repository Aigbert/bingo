//! One shelf per plugin, with serialized compare-and-swap writes.
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use bingo_shelf::{Codec, CodecError, Shelf};
use serde_json::{Value, json};
use crate::codec::{INTERNAL_ERROR, INVALID_PARAMS, RpcError};
use crate::host::StateValue;

pub type Version = u64;

#[derive(Debug, PartialEq, Eq)]
pub struct Conflict(pub Option<Version>);

pub fn swap(held: Option<Version>, if_version: Option<Version>) -> Result<Version, Conflict> {
    if if_version.is_some() && if_version != held {
        return Err(Conflict(held));
    }
    held.unwrap_or_default().checked_add(1).ok_or(Conflict(held))
}

struct Json;
impl Codec for Json {
    type Record = (String, BTreeMap<String, StateValue>);
    const EXT: &'static str = "json";
    fn encode(record: &Self::Record) -> Result<String, CodecError> {
        serde_json::to_string(&record.1).map_err(|e| CodecError(e.to_string()))
    }
    fn decode(id: &str, text: &str) -> Result<Self::Record, CodecError> {
        serde_json::from_str(text).map(|value| (id.into(), value)).map_err(|e| CodecError(e.to_string()))
    }
}

#[derive(Clone)]
pub struct Store {
    root: PathBuf,
    lock: Arc<tokio::sync::Mutex<()>>,
}
impl Store {
    pub fn new(data_dir: &std::path::Path, plugin: &str) -> Self {
        Self { root: data_dir.join("plugins").join(plugin).join("state"), lock: Arc::default() }
    }

    pub async fn get(&self, scope: String, key: String) -> Result<Option<StateValue>, RpcError> {
        self.access(scope, move |_, values| Ok(values.get(&key).cloned())).await
    }

    pub async fn set(&self, scope: String, key: String, value: Value, expected: Option<Version>) -> Result<StateValue, RpcError> {
        self.access(scope, move |(shelf, scope), values| {
            let held = values.get(&key).map(|entry| entry.version);
            let version = swap(held, expected).map_err(|Conflict(version)| {
                let mut error = RpcError::new(-32009, "plugin state version conflict");
                error.data = Some(json!({"version": version}));
                error
            })?;
            let entry = StateValue { version, value };
            values.insert(key, entry.clone());
            shelf.save(scope, &(scope.into(), values.clone())).map_err(failed)?;
            Ok(entry)
        }).await
    }

    pub async fn remove(&self, scope: String) -> Result<(), RpcError> {
        self.access(scope, |(shelf, scope), _| shelf.delete(scope).map_err(failed)).await
    }

    async fn access<T: Send + 'static>(&self, scope: String,
        operation: impl FnOnce((&Shelf<Json>, &str), &mut BTreeMap<String, StateValue>) -> Result<T, RpcError> + Send + 'static,
    ) -> Result<T, RpcError> {
        validate(&scope)?;
        let guard = self.lock.clone().lock_owned().await;
        let root = self.root.clone();
        tokio::task::spawn_blocking(move || {
            let _guard = guard;
            let shelf = Shelf::<Json>::new(root);
            let loaded = shelf.load();
            if let Some(error) = loaded.unreadable.iter().find(|error| error.file == format!("{scope}.json")) {
                return Err(failed(&error.why));
            }
            let mut values = loaded.entries.into_iter().find(|(id, _)| id == &scope)
                .map(|(_, values)| values).unwrap_or_default();
            operation((&shelf, &scope), &mut values)
        }).await.map_err(failed)?
    }
}
fn validate(scope: &str) -> Result<(), RpcError> {
    if !scope.is_empty() && scope.len() <= 120 && scope.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-') {
        return Ok(());
    }
    Err(RpcError::new(INVALID_PARAMS, "invalid plugin state scope"))
}
fn failed(error: impl std::fmt::Display) -> RpcError {
    RpcError::new(INTERNAL_ERROR, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compare_and_swap_only_accepts_the_held_version() {
        assert_eq!(swap(None, None), Ok(1));
        assert_eq!(swap(Some(1), None), Ok(2));
        assert_eq!(swap(Some(1), Some(1)), Ok(2));
        assert_eq!(swap(None, Some(0)), Err(Conflict(None)));
        assert_eq!(swap(Some(2), Some(1)), Err(Conflict(Some(2))));
        assert_eq!(swap(Some(u64::MAX), None), Err(Conflict(Some(u64::MAX))));
    }
}
