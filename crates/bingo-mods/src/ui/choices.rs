//! The person's screen preferences belong to their data, outside mod state.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Default, Serialize, Deserialize)]
#[serde(transparent)]
struct Choices(BTreeMap<String, BTreeMap<String, bool>>);

pub(super) struct Memory {
    path: PathBuf,
    lock: std::sync::Mutex<()>,
}
impl Memory {
    pub(super) fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Default::default(),
        }
    }
    fn read(&self) -> Choices {
        std::fs::read(&self.path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }
    pub(super) fn open(&self, plugin: &str, pane: &str) -> bool {
        self.read()
            .0
            .get(plugin)
            .and_then(|panes| panes.get(pane))
            .copied()
            .unwrap_or(false)
    }
    pub(super) fn remember(&self, plugin: &str, pane: &str, open: bool) -> std::io::Result<()> {
        let _guard = self.lock.lock().unwrap_or_else(|error| error.into_inner());
        let mut choices = self.read();
        choices
            .0
            .entry(plugin.into())
            .or_default()
            .insert(pane.into(), open);
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, serde_json::to_vec(&choices)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choice_format_is_nested_by_plugin_and_pane() {
        let fixture = r#"{"counter":{"main":true,"detail":false}}"#;
        let choices: Choices = serde_json::from_str(fixture).unwrap();
        assert!(choices.0["counter"]["main"]);
        assert!(!choices.0["counter"]["detail"]);
        assert_eq!(
            serde_json::to_value(choices).unwrap(),
            serde_json::from_str::<serde_json::Value>(fixture).unwrap()
        );
    }

    #[test]
    fn restart_keeps_choices_and_bad_files_mean_waiting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pane-choices.json");
        let memory = Memory::new(path.clone());
        assert!(!memory.open("counter", "main"));
        memory.remember("counter", "main", true).unwrap();
        assert!(Memory::new(path.clone()).open("counter", "main"));
        memory.remember("counter", "main", false).unwrap();
        assert!(!Memory::new(path.clone()).open("counter", "main"));
        std::fs::write(&path, "broken").unwrap();
        assert!(!Memory::new(path).open("counter", "main"));
    }
}
