//! Facts the directory fetches for the rows on screen only: whether another
//! process holds a session open, and what a fork's unlisted source is called.
//! Drawing never waits on them; an answer redraws the directory.
use crate::rows::{Row, Target};
use bingo_sdk::{component::SessionsProps, *};
use std::collections::BTreeMap;

#[derive(Default)]
pub(crate) struct Probed {
    /// Answered or in flight; only `true` is drawn.
    pub elsewhere: BTreeMap<SessionId, bool>,
    /// A fork source's label, `None` while in flight or when nothing answers.
    pub sources: BTreeMap<SessionId, Option<String>>,
}

#[derive(Default)]
pub(crate) struct Wanted {
    elsewhere: Vec<SessionId>,
    sources: Vec<SessionId>,
}

/// The probes the visible rows still need, claimed so each runs once.
pub(crate) fn wanted(probed: &mut Probed, visible: &[&Row], props: &SessionsProps) -> Wanted {
    let page = &props.sessions[..props.limit.min(props.sessions.len())];
    let mut wanted = Wanted::default();
    for row in visible {
        let Target::Session(id) = &row.target else {
            continue;
        };
        if id != &props.session && !probed.elsewhere.contains_key(id) {
            probed.elsewhere.insert(id.clone(), false);
            wanted.elsewhere.push(id.clone());
        }
        let source = page
            .iter()
            .find(|summary| &summary.id == id)
            .and_then(|summary| summary.history.forked_from.as_ref())
            .map(|link| &link.session);
        if let Some(source) = source
            && !page.iter().any(|summary| &summary.id == source)
            && !probed.sources.contains_key(source)
        {
            probed.sources.insert(source.clone(), None);
            wanted.sources.push(source.clone());
        }
    }
    wanted
}

/// Asks the host off the drawing path, then redraws the directory.
pub(crate) fn start(
    shared: crate::Shared,
    host: HostHandle,
    (owner, request): (SessionId, String),
    wanted: Wanted,
    now: jiff::Timestamp,
) {
    if wanted.elsewhere.is_empty() && wanted.sources.is_empty() {
        return;
    }
    tokio::spawn(async move {
        let answered = answer(&host, wanted, now).await;
        if let Ok(mut state) = shared.lock()
            && let Some(cursor) = state.cursors.get_mut(&(owner.clone(), request.clone()))
        {
            cursor.probed.elsewhere.extend(answered.elsewhere);
            cursor.probed.sources.extend(answered.sources);
        }
        let _ = host.invalidate(&owner, vec![request]).await;
    });
}

/// A probe that fails says nothing: no badge, and the short id stays.
async fn answer(host: &HostHandle, wanted: Wanted, now: jiff::Timestamp) -> Probed {
    let mut answered = Probed::default();
    for id in wanted.elsewhere {
        let open = host.open_elsewhere(&id).await.unwrap_or(false);
        answered.elsewhere.insert(id, open);
    }
    for id in wanted.sources {
        let label = host
            .summary(&id)
            .await
            .ok()
            .map(|summary| crate::verbs::label(&summary, now));
        answered.sources.insert(id, label);
    }
    answered
}

/// The tail of an id: the time-ordered head is shared by sessions made
/// together, the random tail tells them apart.
pub(crate) fn short(id: &SessionId) -> String {
    let chars: Vec<char> = id.as_str().chars().collect();
    chars[chars.len().saturating_sub(8)..].iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn probes_run_once_per_visible_row_and_failures_say_nothing() {
        let mut copy = crate::tests::summary("copy", Some("Copy"));
        copy.history.forked_from = Some(ForkLink {
            session: SessionId::from_raw("gone"),
            turn: None,
        });
        let mut props = crate::tests::props();
        props.sessions = vec![copy, crate::tests::summary("held", Some("Held"))];
        let rows = crate::rows::listed(&props, &crate::state::Cursor::default());
        let visible: Vec<&Row> = rows.iter().collect();
        let mut probed = Probed::default();
        let wanted = wanted(&mut probed, &visible, &props);
        assert_eq!(wanted.elsewhere.len(), 2);
        assert_eq!(wanted.sources, [SessionId::from_raw("gone")]);
        let again = super::wanted(&mut probed, &visible, &props);
        assert!(again.elsewhere.is_empty() && again.sources.is_empty());

        let host = testing::TestHost::default();
        host.on_open_elsewhere(|id| async move { Ok(id.as_str() == "held") });
        host.on_summary(|_| async { Err(KernelError::new(ErrorCode::SessionNotFound, "gone")) });
        let answered = answer(&host.handle(), wanted, props.now).await;
        assert_eq!(
            answered.elsewhere.get(&SessionId::from_raw("held")),
            Some(&true)
        );
        assert_eq!(
            answered.elsewhere.get(&SessionId::from_raw("copy")),
            Some(&false)
        );
        assert_eq!(
            answered.sources.get(&SessionId::from_raw("gone")),
            Some(&None)
        );
        assert_eq!(
            short(&SessionId::from_raw("ses_01JABCDEFGH12345678")),
            "12345678"
        );
    }
}
