use bingo_sdk::*;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Target {
    Session(SessionId),
    Group(String),
    More,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub target: Target,
    pub title: String,
    pub first_ask: Option<String>,
    pub provenance: Option<String>,
    pub detail: String,
    pub depth: usize,
    pub current: bool,
}
pub fn rows(
    summaries: &[SessionSummary],
    now: jiff::Timestamp,
    query: &str,
    current: &SessionId,
) -> Vec<Row> {
    let probed = crate::probe::Probed::default();
    expanded(summaries, now, query, current, &BTreeSet::new(), &probed)
}
pub(crate) fn expanded(
    summaries: &[SessionSummary],
    now: jiff::Timestamp,
    query: &str,
    current: &SessionId,
    expanded: &BTreeSet<String>,
    probed: &crate::probe::Probed,
) -> Vec<Row> {
    let query = query.to_lowercase();
    let projection = Projection {
        all: summaries,
        query: &query,
        now,
        current,
        probed,
    };
    let forest = forest(summaries);
    let mut out = Vec::new();
    let mut visited = BTreeSet::new();
    for root in forest.roots {
        projection.branch(root, 0, &mut visited, &mut out);
    }
    for (name, members) in forest.groups {
        let mut children = Vec::new();
        for member in members {
            projection.branch(member, 1, &mut visited, &mut children);
        }
        if children.is_empty() {
            continue;
        }
        let open = expanded.contains(&name) || !query.is_empty();
        out.push(group(name, children.len(), open));
        if open {
            out.extend(children);
        }
    }
    // Corrupt parent cycles still leave their members discoverable.
    for summary in summaries {
        if !visited.contains(&summary.id) {
            projection.branch(summary, 0, &mut visited, &mut out);
        }
    }
    out
}
struct Forest<'a> {
    roots: Vec<&'a SessionSummary>,
    groups: BTreeMap<String, Vec<&'a SessionSummary>>,
}
fn forest(summaries: &[SessionSummary]) -> Forest<'_> {
    let ids: BTreeSet<_> = summaries.iter().map(|s| &s.id).collect();
    let mut forest = Forest {
        roots: Vec::new(),
        groups: BTreeMap::new(),
    };
    for summary in summaries {
        if summary
            .parent
            .as_ref()
            .is_some_and(|p| ids.contains(&p.session))
        {
            continue;
        }
        if let Some(key) = &summary.key {
            forest
                .groups
                .entry(key.split('/').next().unwrap_or(key).into())
                .or_default()
                .push(summary);
        } else {
            forest.roots.push(summary);
        }
    }
    forest
        .roots
        .sort_by_key(|s| std::cmp::Reverse(s.updated_at));
    forest
}
fn group(name: String, count: usize, open: bool) -> Row {
    Row {
        title: format!("{name} ({count})"),
        target: Target::Group(name),
        first_ask: None,
        provenance: None,
        detail: if open { "▾" } else { "▸" }.into(),
        depth: 0,
        current: false,
    }
}
struct Projection<'a> {
    all: &'a [SessionSummary],
    query: &'a str,
    now: jiff::Timestamp,
    current: &'a SessionId,
    probed: &'a crate::probe::Probed,
}
impl Projection<'_> {
    fn branch(
        &self,
        summary: &SessionSummary,
        depth: usize,
        visited: &mut BTreeSet<SessionId>,
        out: &mut Vec<Row>,
    ) {
        if !visited.insert(summary.id.clone()) {
            return;
        }
        let mut children = Vec::new();
        for child in self
            .all
            .iter()
            .filter(|s| s.parent.as_ref().is_some_and(|p| p.session == summary.id))
        {
            self.branch(child, depth + 1, visited, &mut children);
        }
        if matches(summary, self.query) || !children.is_empty() {
            let mut row = row(summary, self.now, self.current, depth);
            if self.probed.elsewhere.get(&summary.id) == Some(&true)
                && !summary.asking
                && !summary.busy
            {
                row.detail.push_str("   ● open elsewhere");
            }
            row.provenance = summary
                .history
                .forked_from
                .as_ref()
                .map(|link| format!("⑂ from {}", self.source(&link.session)));
            out.push(row);
        }
        out.extend(children);
    }
}
impl Projection<'_> {
    /// A source on this page names itself; one off it is looked up on
    /// screen, and until then, or if it is gone, its short id stands in.
    fn source(&self, id: &SessionId) -> String {
        self.all
            .iter()
            .find(|summary| &summary.id == id)
            .map(|summary| crate::verbs::label(summary, self.now))
            .or_else(|| self.probed.sources.get(id).cloned().flatten())
            .unwrap_or_else(|| crate::probe::short(id))
    }
}
fn matches(summary: &SessionSummary, query: &str) -> bool {
    query.is_empty()
        || summary
            .title
            .iter()
            .chain(summary.history.first_ask.iter())
            .any(|s| s.to_lowercase().contains(query))
}
fn row(summary: &SessionSummary, now: jiff::Timestamp, current: &SessionId, depth: usize) -> Row {
    let title = summary
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
        .unwrap_or_else(|| {
            summary
                .cwd
                .rsplit(['/', '\\'])
                .find(|part| !part.is_empty())
                .unwrap_or(&summary.cwd)
                .to_owned()
        });
    let first_ask = summary
        .history
        .first_ask
        .clone()
        .filter(|ask| ask != &title);
    let mut detail = ago(now, summary.updated_at);
    if let Some(n) = summary.history.messages {
        detail.push_str(&format!("   {n} msg"));
    }
    if summary.asking {
        detail.push_str("   ● asking");
    } else if summary.busy {
        detail.push_str("   ● busy");
    }
    Row {
        target: Target::Session(summary.id.clone()),
        title,
        first_ask,
        provenance: None,
        detail,
        depth,
        current: &summary.id == current,
    }
}
pub(crate) fn listed(props: &component::SessionsProps, cursor: &crate::state::Cursor) -> Vec<Row> {
    let end = props.limit.min(props.sessions.len());
    let mut rows = expanded(
        &props.sessions[..end],
        props.now,
        &props.query,
        &props.session,
        &cursor.expanded,
        &cursor.probed,
    );
    if props.sessions.len() > props.limit {
        rows.push(Row {
            target: Target::More,
            title: "more…".into(),
            first_ask: None,
            provenance: None,
            detail: "50 at a time".into(),
            depth: 0,
            current: false,
        });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::summary;
    #[test]
    fn tree_groups_first_asks_case_fold_and_activity() {
        let mut root = summary("root", Some("Fix the race"));
        root.asking = true;
        root.history.messages = Some(38);
        let mut child = summary("child", Some("review: FIFO"));
        child.parent = Some(ParentLink {
            session: root.id.clone(),
            item: None,
        });
        let mut fixture = summary("fixture", Some("checkpoint v2"));
        fixture.history.first_ask = Some("can you add a fixture?".into());
        let mut scheduled = summary("scheduled", None);
        scheduled.key = Some("schedule/id".into());
        let all = [root, child, fixture, scheduled];
        let now = "2026-10-15T12:00:00Z".parse().unwrap();
        let rows = rows(&all, now, "", &SessionId::from_raw("root"));
        assert_eq!(
            rows.iter().map(|r| r.title.as_str()).collect::<Vec<_>>(),
            [
                "Fix the race",
                "review: FIFO",
                "checkpoint v2",
                "schedule (1)"
            ]
        );
        assert_eq!(rows[1].depth, 1);
        assert_eq!(rows[0].detail, "2h ago   38 msg   ● asking");
        assert!(rows[0].current);
        assert_eq!(rows[2].first_ask.as_deref(), Some("can you add a fixture?"));
        assert_eq!(super::rows(&all, now, "ADD A FIX", &all[0].id).len(), 1);
        assert_eq!(
            super::rows(&all, now, "FIFO", &all[0].id).len(),
            2,
            "a matched child retains its parent"
        );
    }
    #[test]
    fn provenance_and_confirmation_labels_never_expose_ids() {
        let source = summary("raw-source-id", Some("Original"));
        let mut copy = summary("raw-copy-id", Some("Copy"));
        copy.history.forked_from = Some(ForkLink {
            session: source.id.clone(),
            turn: None,
        });
        let mut unnamed = summary("secret-id", Some(""));
        unnamed.history.first_ask = Some("First ask".into());
        assert_eq!(
            crate::verbs::label(&unnamed, unnamed.updated_at),
            "First ask"
        );
        let now = "2026-10-15T12:00:00Z".parse().unwrap();
        let rows = rows(&[source, copy], now, "", &SessionId::from_raw("none"));
        assert_eq!(rows[1].provenance.as_deref(), Some("⑂ from Original"));
        assert_eq!(
            crate::verbs::label(&summary("secret-id", None), now),
            "untitled · 2h ago"
        );
    }
}
