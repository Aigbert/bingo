//! The branch belongs to the session's directory, never the surface process.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;

use async_trait::async_trait;
use bingo_sdk::{Event, Frame, Hook, HookContext, HookMatcher, HookPoint, Phase, SessionId};

#[derive(Default)]
pub(crate) struct Branch {
    directories: Mutex<HashMap<SessionId, PathBuf>>,
}

impl Branch {
    async fn publish(&self, cwd: &Path, cx: &HookContext) {
        let changed = match self.directories.lock() {
            Ok(mut directories) => {
                if directories
                    .get(&cx.session)
                    .is_some_and(|previous| previous == cwd)
                {
                    false
                } else {
                    directories.insert(cx.session.clone(), cwd.to_path_buf());
                    true
                }
            }
            Err(_) => false,
        };
        if changed
            && let Err(error) = cx
                .host
                .extend(&cx.session, "_bingo.context", "location", read(cwd).await)
                .await
        {
            tracing::warn!(%error, "context: branch was not published");
        }
    }
}

async fn read(cwd: &Path) -> serde_json::Value {
    let branch = tokio::process::Command::new("git")
        .args(["symbolic-ref", "--quiet", "--short", "HEAD"])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .output()
        .await
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty());
    bingo_sdk::slot::with(
        serde_json::json!({}),
        [bingo_sdk::slot::Slot::Location {
            cwd: cwd.display().to_string(),
            branch,
        }],
    )
}

#[async_trait]
impl Hook for Branch {
    fn id(&self) -> &str {
        "context:branch"
    }

    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![HookPoint::Session, HookPoint::Event],
            tool: None,
        }
    }

    async fn on_session(&self, phase: Phase, cx: &HookContext) {
        if phase == Phase::Start {
            self.publish(&cx.cwd, cx).await;
        }
    }

    async fn on_event(&self, frame: &Frame, cx: &HookContext) {
        if let Event::SessionUpdated { summary } = &frame.event {
            self.publish(Path::new(&summary.cwd), cx).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_branch_is_a_cwd_fact_and_detached_or_non_git_is_absent() {
        let Some(repo) = crate::git::Repo::init() else {
            return;
        };
        let path = repo.root();
        let fact = read(&path).await;
        assert_eq!(fact["slots"][0]["cwd"], path.to_string_lossy().as_ref());
        assert!(
            fact["slots"][0]["branch"]
                .as_str()
                .is_some_and(|s| !s.is_empty())
        );
        let status = std::process::Command::new("git")
            .args(["checkout", "--detach", "--quiet"])
            .current_dir(&path)
            .status()
            .unwrap();
        assert!(status.success());
        assert!(read(&path).await["slots"][0]["branch"].is_null());
        let outside = tempfile::tempdir().unwrap();
        assert!(read(outside.path()).await["slots"][0]["branch"].is_null());
    }
}
