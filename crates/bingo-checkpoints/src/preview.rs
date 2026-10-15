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
    async fn touched(&self, session: &SessionId, from: &TurnId) -> Result<Touched, ServiceError> {
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
        let undone =
            transcript::from(&turns, from).ok_or_else(|| error("this session has no such turn"))?;
        let plan = crate::restore::plan(&self.store, session, &undone).map_err(error)?;
        let shell_calls = state
            .transcript
            .items()
            .iter()
            .filter(|item| {
                item.turn.as_ref().is_some_and(|id| undone.contains(id))
                    && matches!(&item.body, ItemBody::ToolCall { name, .. } if name == "Bash")
            })
            .count();
        Ok(project(plan, shell_calls))
    }
}
fn error(e: impl std::fmt::Display) -> ServiceError {
    ServiceError(e.to_string())
}
fn project(plan: crate::restore::Plan, shell_calls: usize) -> Touched {
    let mut files: Vec<_> = plan
        .put_back
        .into_iter()
        .map(|(path, _)| File {
            path,
            fate: Fate::Restored,
        })
        .collect();
    files.extend(plan.remove.into_iter().map(|path| File {
        path,
        fate: Fate::Removed,
    }));
    files.extend(plan.skipped.into_iter().map(|path| File {
        path,
        fate: Fate::Skipped,
    }));
    Touched { files, shell_calls }
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
            output: Some(bingo_sdk::ToolOutput::text("hello")),
            progress: None,
            duration_ms: None,
            quiet: false,
        };
        state
            .transcript
            .apply(&bingo_sdk::Event::ItemUpdated { item: call });
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
            .unwrap();
        assert_eq!(touched.shell_calls, 1);
        assert_eq!(
            touched.files,
            [
                File {
                    path: old.clone(),
                    fate: Fate::Restored
                },
                File {
                    path: new,
                    fate: Fate::Removed
                }
            ]
        );
        assert_eq!(std::fs::read(&old).unwrap(), b"second");
        assert!(fixture.journal.rewound().is_empty());
        assert_eq!(
            preview
                .touched(&crate::tests::session(), &TurnId::from_raw("trn_2"))
                .await
                .unwrap()
                .shell_calls,
            0
        );
        assert!(
            preview
                .touched(&crate::tests::session(), &TurnId::from_raw("missing"))
                .await
                .is_err()
        );
    }
}
