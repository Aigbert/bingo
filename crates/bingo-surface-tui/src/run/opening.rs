//! The most recent session offered at the welcome site.
use super::Run;

/// Read the same cwd's most recent earlier root once, on the host-call lane.
pub(super) fn find_resume(run: &mut Run) {
    let host = run.host.clone();
    let state = run.session.tree.root();
    let id = state.summary.id.clone();
    let created = state.summary.created_at;
    let filter = bingo_sdk::SessionFilter {
        cwd: Some(std::path::PathBuf::from(&state.summary.cwd)),
        parent: None,
        limit: None,
    };
    run.spawn(async move {
        host.sessions(filter).await.map(|sessions| {
            super::Reply::Resume(
                id.clone(),
                sessions
                    .into_iter()
                    .filter(|session| {
                        session.id != id
                            && session.parent.is_none()
                            && session.driver == bingo_sdk::Driver::Model
                            && session.created_at < created
                    })
                    .max_by_key(|session| session.updated_at)
                    .map(Box::new),
            )
        })
    });
}
