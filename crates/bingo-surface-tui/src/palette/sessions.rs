use crate::{
    clock::{self, Now},
    seats::{self, Ear, Owes},
    tree::{self, Tree},
};
use bingo_sdk::{SessionState, SessionSummary, component::PaletteSession};

pub const RECENT: usize = 20;

pub fn recent_roots(mut sessions: Vec<SessionSummary>) -> Vec<SessionSummary> {
    sessions.retain(|session| session.parent.is_none());
    sessions.sort_by_key(|session| std::cmp::Reverse(session.updated_at));
    sessions.truncate(RECENT);
    sessions
}

pub fn stored(summary: &SessionSummary) -> PaletteSession {
    PaletteSession {
        id: summary.id.clone(),
        label: summary
            .title
            .clone()
            .unwrap_or_else(|| tree::directory(&summary.cwd)),
        detail: format!(
            "{}{}",
            summary.updated_at,
            summary
                .messages
                .map(|n| format!(" · {n} messages"))
                .unwrap_or_default()
        ),
        attention: None,
    }
}

pub fn conversation(tree: &Tree, stored: &[SessionSummary], now: Now) -> Vec<PaletteSession> {
    let mut rows = tree::roster(tree, stored);
    rows.sort_by_key(|row| (!tree.is_root(row.session), row.status.is_none()));
    rows.into_iter()
        .map(|row| {
            let Some(state) = tree.state(row.session) else {
                return PaletteSession {
                    id: row.session.clone(),
                    label: row.name,
                    detail: "stored".into(),
                    attention: None,
                };
            };
            let (detail, attention) = if row.status.is_some() {
                member(tree, state, now)
            } else {
                room(tree, state)
            };
            let label = if row.status.is_some() {
                let ear = if seats::seat(tree, state).is_some_and(|seat| seat.ear == Ear::Live) {
                    "~"
                } else {
                    ""
                };
                format!("{}{ear} {}", crate::face::face(&row.name).sigil(), row.name)
            } else {
                row.name
            };
            PaletteSession {
                id: row.session.clone(),
                label,
                detail,
                attention,
            }
        })
        .collect()
}

fn member(tree: &Tree, state: &SessionState, now: Now) -> (String, Option<String>) {
    let mut words = vec![tree::describe(state, "")];
    let mut attention = tree::asking(state).then(|| "needs you".into());
    if let Some(seat) = seats::seat(tree, state) {
        words.push(format!("in {}", seat.room));
        if let Some(unread) = seat.unread.filter(|n| *n > 0) {
            words.push(format!("{unread} unread"));
        }
        match seat.ear {
            Ear::Live => words.push("live".into()),
            Ear::Listening {
                patience_s: Some(seconds),
            } => words.push(format!("listening · {seconds}s")),
            _ => {}
        }
        if let Some(owes) = seat.owes {
            attention = Some(owed(owes, now));
        }
    }
    (
        words
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · "),
        attention,
    )
}
fn owed(owes: Owes, now: Now) -> String {
    match owes {
        Owes::Since(at) => format!("owes an answer · {}", clock::span(now.past(at))),
        Owes::At(at) => format!("owes an answer since {at}"),
    }
}
fn room(tree: &Tree, state: &SessionState) -> (String, Option<String>) {
    let counts = seats::counts(tree, state);
    (
        format!(
            "{} seat{}",
            counts.seats,
            if counts.seats == 1 { "" } else { "s" }
        ),
        (counts.owed > 0).then(|| format!("{} owed", counts.owed)),
    )
}
