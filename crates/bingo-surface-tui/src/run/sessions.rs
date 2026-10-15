use super::*;
use bingo_sdk::component::SessionsScope;
pub(super) fn list(run: &mut Run) {
    let Open::Sessions(open) = run.ui.focus.open() else {
        return;
    };
    let request = listing(open);
    let filter = SessionFilter {
        cwd: (open.scope == SessionsScope::Here)
            .then(|| run.session.tree.viewed().summary.cwd.clone().into()),
        limit: Some(open.limit + 1),
        parent: None,
    };
    let host = run.host.clone();
    run.spawn(async move {
        host.sessions(filter)
            .await
            .map(|rows| Reply::Sessions(request, rows))
    });
}
pub(super) fn fill(run: &mut Run, request: &str, rows: Vec<SessionSummary>) {
    if let Some(Open::Sessions(open)) = run.ui.focus.open_mut()
        && listing(open) == request
    {
        open.sessions = rows;
    }
}
pub(super) fn surface_action(run: &mut Run, action: &bingo_sdk::Action) {
    let now = run.detection.now();
    if action.name == "Sessions" {
        let effects = crate::sessions::open(&mut run.ui, now);
        run.apply(effects);
    }
}

fn listing(open: &crate::sessions::Opened) -> String {
    format!("{}:{:?}:{}", open.id, open.scope, open.limit)
}

pub(super) fn leave(run: &Run, next: &SessionId) {
    let host = run.host.clone();
    let old = run.session.tree.root_id().clone();
    let empty = run.session.bootstrap
        && old != *next
        && run.session.tree.root().summary.history.messages == Some(0)
        && run.session.tree.root().summary.title.is_none()
        && !run.session.tree.root().busy();
    tokio::spawn(async move {
        if empty {
            host.delete(&old).await
        } else {
            host.close(&old, CloseReason::Client).await
        }
    });
}
