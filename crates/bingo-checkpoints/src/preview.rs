use bingo_sdk::{
    ClientIdentity, HostHandle, ItemBody, OpenOptions, ServiceError, SessionId, SessionSelector,
    TurnId, transcript,
};
use bingo_services::checkpoints::{Checkpoints, Fate, File, Touched};
use std::sync::{Arc, OnceLock};
pub struct Preview {
    pub store: Arc<crate::store::Checkpoints>,
    pub host: Arc<OnceLock<HostHandle>>,
}
#[async_trait::async_trait]
impl Checkpoints for Preview {
    async fn touched(
        &self,
        session: &SessionId,
        from: &TurnId,
    ) -> Result<Option<Touched>, ServiceError> {
        let host = self
            .host
            .get()
            .ok_or_else(|| error("checkpoints have not started"))?;
        let attachment = host
            .open(
                SessionSelector::ById {
                    id: session.clone(),
                },
                ClientIdentity {
                    name: "checkpoints".into(),
                    surface: "preview".into(),
                },
                OpenOptions::default(),
            )
            .await
            .map_err(error)?;
        let state = attachment.snapshot;
        let turns = transcript::turns(&state.transcript);
        let Some(undone) = transcript::from(&turns, from) else {
            return Ok(None);
        };
        let first = turns
            .iter()
            .find(|turn| &turn.id == from)
            .map_or(0, |turn| turn.first);
        let shell_calls = state.transcript.items()[first..]
            .iter()
            .filter(|item| {
                matches!(&item.body, ItemBody::Shell { .. })
                    || matches!(&item.body, ItemBody::ToolCall { name, .. } if name == "Bash")
            })
            .count();
        let files = crate::restore::paths(&self.store, session, &undone)
            .into_iter()
            .map(|(_, entry)| File {
                changed: self.store.changed(session, &undone, &entry.path),
                path: entry.path,
                fate: match entry.state {
                    crate::store::State::Present => Fate::Restored,
                    crate::store::State::Absent => Fate::Removed,
                    crate::store::State::Skipped => Fate::Skipped,
                },
            })
            .collect();
        Ok(Some(Touched { files, shell_calls }))
    }
}
fn error(e: impl std::fmt::Display) -> ServiceError {
    ServiceError(e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn preview_uses_oldest_snapshots_and_counts_bash_without_writing() {
        let mut fixture = crate::tests::Fixture::new();
        let mut state = crate::tests::transcript();
        let mut call = state.transcript.items()[1].as_ref().clone();
        call.body = ItemBody::ToolCall {
            call_id: "bash".into(),
            name: "Bash".into(),
            input: serde_json::json!({"command":"echo hello"}),
            output: Some(Box::new(bingo_sdk::ToolOutput::text("hello"))),
            progress: None,
            duration_ms: None,
            allowed_by: None,
            asked_at: None,
            began_at: None,
            quiet: false,
        };
        state
            .transcript
            .apply(&bingo_sdk::Event::ItemUpdated { item: call });
        let mut shell = state.transcript.items()[1].as_ref().clone();
        shell.id = bingo_sdk::ItemId::from_raw("shell");
        shell.turn = None;
        shell.body = ItemBody::Shell {
            command: "echo local".into(),
            cwd: fixture.cwd.clone(),
            output: "local".into(),
            exit: Some(0),
        };
        let mut items: Vec<_> = state
            .transcript
            .items()
            .iter()
            .map(|i| i.as_ref().clone())
            .collect();
        items.insert(2, shell);
        state.transcript = items.into_iter().collect();
        fixture.journal = crate::tests::Journal::holding(state);
        std::fs::write(fixture.cwd.join("old"), "before").unwrap();
        let old = fixture.edit("trn_1", "old", b"first");
        fixture.edit("trn_2", "old", b"second");
        let new = fixture.edit("trn_2", "new", b"new");
        let host = Arc::new(OnceLock::new());
        host.set(fixture.journal.handle()).ok().unwrap();
        let preview = Preview {
            store: fixture.store.clone(),
            host,
        };
        let touched = preview
            .touched(&crate::tests::session(), &TurnId::from_raw("trn_1"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(touched.shell_calls, 2);
        assert_eq!(
            touched.files,
            [
                File {
                    path: old.clone(),
                    fate: Fate::Restored,
                    changed: false
                },
                File {
                    path: new,
                    fate: Fate::Removed,
                    changed: false
                }
            ]
        );
        assert_eq!(std::fs::read(&old).unwrap(), b"second");
        std::fs::write(&old, "my manual edit").unwrap();
        std::fs::remove_file(fixture.store.root().join("ses_one/trn_1/1.snap")).unwrap();
        let changed = preview
            .touched(&crate::tests::session(), &TurnId::from_raw("trn_1"))
            .await
            .unwrap()
            .unwrap();
        assert!(
            changed.files[0].changed,
            "preview marks edits without reading snapshot bytes"
        );
        assert!(fixture.journal.rewound().is_empty());
        assert_eq!(
            preview
                .touched(&crate::tests::session(), &TurnId::from_raw("trn_2"))
                .await
                .unwrap()
                .unwrap()
                .shell_calls,
            0
        );
        assert_eq!(
            preview
                .touched(&crate::tests::session(), &TurnId::from_raw("missing"))
                .await
                .unwrap(),
            None,
            "a turn the session lacks is an answer, not an error"
        );
    }
}
