//! The post-turn fingerprint is separate from the pre-turn snapshot index.
use crate::store::{Checkpoints, MOST};
use bingo_sdk::{ErrorCode, KernelError, SessionId, TurnId};
use std::{collections::BTreeMap, path::Path};

fn fingerprint(path: &Path) -> String {
    match std::fs::metadata(path) {
        Ok(meta) if meta.is_file() && meta.len() <= MOST => std::fs::read(path)
            .map(|bytes| bingo_digest::hex(&bingo_digest::digest(&bytes)))
            .unwrap_or_else(|_| "unreadable".into()),
        Ok(_) => "untracked".into(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => "absent".into(),
        Err(_) => "unreadable".into(),
    }
}

impl Checkpoints {
    pub fn record_after(&self, session: &SessionId, turn: &TurnId) -> Result<(), KernelError> {
        let _held = self.writing.lock().unwrap_or_else(|e| e.into_inner());
        let entries = self.entries(session, turn);
        if entries.is_empty() {
            return Ok(());
        }
        let hashes: BTreeMap<_, _> = entries
            .iter()
            .map(|entry| {
                (
                    entry.path.to_string_lossy().into_owned(),
                    fingerprint(&entry.path),
                )
            })
            .collect();
        let text = serde_json::to_vec(&hashes).map_err(storage)?;
        std::fs::write(self.turn_dir(session, turn).join("after.json"), text).map_err(storage)
    }

    pub fn changed(&self, session: &SessionId, turns: &[TurnId], path: &Path) -> bool {
        for turn in turns.iter().rev() {
            if !self
                .entries(session, turn)
                .iter()
                .any(|entry| entry.path == path)
            {
                continue;
            }
            let hashes: BTreeMap<String, String> =
                std::fs::read(self.turn_dir(session, turn).join("after.json"))
                    .ok()
                    .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                    .unwrap_or_default();
            return hashes
                .get(path.to_string_lossy().as_ref())
                .is_none_or(|kept| kept != &fingerprint(path));
        }
        false
    }
}
fn storage(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::Storage, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manual_edits_differ_from_the_last_turn_even_after_reopening() {
        let dir = tempfile::tempdir().unwrap();
        let store = Checkpoints::new(dir.path());
        let path = dir.path().join("a.txt");
        let session = SessionId::from_raw("s");
        let turn = TurnId::from_raw("t");
        std::fs::write(&path, "v1").unwrap();
        store.snapshot(&session, &turn, &path).unwrap();
        std::fs::write(&path, "v2").unwrap();
        store.record_after(&session, &turn).unwrap();
        assert!(!store.changed(&session, std::slice::from_ref(&turn), &path));
        std::fs::write(&path, "my manual edit").unwrap();
        let reopened = Checkpoints::new(dir.path());
        assert!(reopened.changed(&session, &[turn], &path));
    }
}
