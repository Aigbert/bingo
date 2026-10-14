use bingo_sdk::*;
use std::path::PathBuf;
pub struct Lifecycle {
    pub path: PathBuf,
    pub state: crate::Shared,
}
#[async_trait::async_trait]
impl Hook for Lifecycle {
    fn id(&self) -> &str {
        "palette.lifecycle"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![HookPoint::Event],
            tool: None,
        }
    }
    async fn on_event(&self, frame: &Frame, cx: &HookContext) {
        if matches!(frame.event, Event::SessionClosed { .. })
            && let Ok(mut state) = self.state.lock()
        {
            state
                .cursors
                .retain(|(session, _), _| session != &frame.session);
        }
        if !matches!(frame.event, Event::SessionUpdated { .. }) || !claim(&self.path) {
            return;
        }
        let Ok(until) = frame.ts.checked_add(std::time::Duration::from_secs(8)) else {
            return;
        };
        let slot = slot::Slot::Toast {
            text: "Sessions now open with ctrl+r; ctrl+g returns to a pending ask".into(),
            until,
            alert: false,
        };
        let _ = cx
            .host
            .signal(
                &frame.session,
                "palette",
                "key-hint",
                slot::with(serde_json::json!({}), [slot]),
            )
            .await;
    }
}
fn claim(path: &std::path::Path) -> bool {
    if let Some(parent) = path.parent()
        && std::fs::create_dir_all(parent).is_err()
    {
        return false;
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .is_ok()
}
#[cfg(test)]
mod tests {
    #[test]
    fn migration_hint_is_claimed_once_in_the_plugin_store() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("palette/key-hint");
        assert!(super::claim(&path));
        assert!(!super::claim(&path));
    }
}
