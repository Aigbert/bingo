//! The last list call, kept only for this process.
use jiff::Timestamp;
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Mutex;

#[derive(Clone, Default, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(super) enum Reach {
    #[default]
    Unchecked,
    Ok {
        ms: u64,
        at: Timestamp,
    },
    Down {
        error: String,
        at: Timestamp,
    },
}

#[derive(Default)]
pub(super) struct Reaches(Mutex<BTreeMap<String, Reach>>);
impl Reaches {
    pub fn get(&self, name: &str) -> Reach {
        self.0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(name)
            .cloned()
            .unwrap_or_default()
    }
    pub fn record(&self, name: &str, reach: Reach) {
        self.0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(name.into(), reach);
    }
    pub fn remove(&self, name: &str) {
        self.0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(name);
    }
}
