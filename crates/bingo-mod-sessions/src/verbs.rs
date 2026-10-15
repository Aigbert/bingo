use bingo_sdk::*;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Deletion {
    pub id: SessionId,
    pub label: String,
    pub blocked: Option<String>,
}

pub(crate) async fn rename(
    host: &HostHandle,
    id: &SessionId,
    title: &str,
) -> Result<(), KernelError> {
    host.reconfigure(id, SessionChange::Title(title.trim().to_owned()))
        .await
}
pub(crate) async fn descendants(
    host: &HostHandle,
    root: &SessionId,
    current: &SessionId,
) -> Result<Vec<Deletion>, KernelError> {
    let summaries = host.sessions(SessionFilter::default()).await?;
    let mut visited = BTreeSet::new();
    let mut out = Vec::new();
    postorder(root, &summaries, &mut visited, &mut out);
    if out.iter().any(|row| &row.id == current) {
        return Err(KernelError::new(
            ErrorCode::InvalidInput,
            "switch to another session before deleting this conversation",
        ));
    }
    for row in &mut out {
        row.blocked = host
            .check_delete(&row.id)
            .await
            .err()
            .map(|error| error.message);
    }
    Ok(out)
}
fn postorder(
    id: &SessionId,
    summaries: &[SessionSummary],
    visited: &mut BTreeSet<SessionId>,
    out: &mut Vec<Deletion>,
) {
    if !visited.insert(id.clone()) {
        return;
    }
    for child in summaries
        .iter()
        .filter(|s| s.parent.as_ref().is_some_and(|p| &p.session == id))
    {
        postorder(&child.id, summaries, visited, out);
    }
    let label = summaries
        .iter()
        .find(|s| &s.id == id)
        .map(|summary| label(summary, jiff::Timestamp::now()))
        .unwrap_or_else(|| "untitled".into());
    out.push(Deletion {
        id: id.clone(),
        label,
        blocked: None,
    });
}
pub(crate) async fn delete(host: &HostHandle, doomed: &[Deletion]) -> Result<(), KernelError> {
    if let Some(reason) = doomed.iter().find_map(|row| row.blocked.as_ref()) {
        return Err(KernelError::new(ErrorCode::NotReady, reason.clone()));
    }
    if let Some(root) = doomed.last() {
        host.delete(&root.id).await?;
    }
    Ok(())
}

pub(crate) fn label(summary: &SessionSummary, now: jiff::Timestamp) -> String {
    summary
        .title
        .clone()
        .filter(|text| !text.trim().is_empty())
        .or_else(|| {
            summary
                .history
                .first_ask
                .clone()
                .filter(|text| !text.trim().is_empty())
        })
        .unwrap_or_else(|| format!("untitled · {}", ago(now, summary.updated_at)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn confirmation_marks_a_blocked_parent_and_deletes_nothing() {
        let parent = crate::tests::summary("parent", Some("Parent"));
        let mut child = crate::tests::summary("child", None);
        child.parent = Some(ParentLink {
            session: parent.id.clone(),
            item: None,
        });
        let host = testing::TestHost::with_sessions(vec![parent.clone(), child]);
        host.on_check_delete(|id| async move {
            if id.as_str() == "parent" {
                Err(KernelError::new(ErrorCode::SessionLocked, "open elsewhere"))
            } else {
                Ok(())
            }
        });
        let rows = descendants(&host.handle(), &parent.id, &SessionId::from_raw("current"))
            .await
            .unwrap();
        assert_eq!(rows[1].blocked.as_deref(), Some("open elsewhere"));
        assert!(rows[0].label.starts_with("untitled · "));
        assert_eq!(
            delete(&host.handle(), &rows).await.unwrap_err().code,
            ErrorCode::NotReady
        );
        assert!(
            !host
                .calls()
                .iter()
                .any(|call| matches!(call, testing::HostCall::Delete { .. }))
        );
        let cursor = crate::state::Cursor {
            sheet: crate::state::Sheet::Delete(rows),
            ..Default::default()
        };
        let view = crate::draw::tree(&crate::tests::props(), &[], &cursor, "list", 20, 80);
        assert!(view.fold().contains("Parent · open elsewhere"));
    }
}
