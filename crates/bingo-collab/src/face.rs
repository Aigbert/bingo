//! A teammate's ink follows creation order in the home tree, including stored rows.
use bingo_sdk::{SessionId, SessionSummary};

const CREW: [&str; 6] = ["crew1", "crew2", "crew0", "crew3", "crew4", "crew5"];

pub fn ink(id: &SessionId, home: &str, rows: &[SessionSummary]) -> &'static str {
    let Some(row) = rows.iter().find(|row| &row.id == id) else {
        return "muted";
    };
    if crate::room::is_room(row) {
        return "plan";
    }
    if row.id.as_str() == home {
        return "glow";
    }
    teammates(home, rows)
        .position(|row| &row.id == id)
        .map_or("muted", |slot| CREW[slot % CREW.len()])
}

/// The home tree's teammates, in the order they were created: the sessions
/// home itself started that are not rooms. A teammate's own hire is not one.
pub fn teammates<'a>(
    home: &str,
    rows: &'a [SessionSummary],
) -> impl Iterator<Item = &'a SessionSummary> {
    let mut crew = rows
        .iter()
        .filter(|row| {
            row.parent.as_ref().map(|parent| parent.session.as_str()) == Some(home)
                && !crate::room::is_room(row)
        })
        .collect::<Vec<_>>();
    crew.sort_by(|a, b| {
        a.created_at
            .cmp(&b.created_at)
            .then_with(|| a.id.cmp(&b.id))
    });
    crew.into_iter()
}

pub fn who(name: &str, home: &str, rows: &[SessionSummary]) -> &'static str {
    match name {
        "bingo" => "glow",
        "you" => "fg",
        name => rows
            .iter()
            .find(|row| crate::name::of_session(row) == name)
            .map_or("muted", |row| ink(&row.id, home, rows)),
    }
}

/// The accent a session's own view wears: a teammate's ink inside its
/// session, ember (`glow`) at home, in a room and anywhere else.
pub fn accent(id: &SessionId, home: &str, rows: &[SessionSummary]) -> &'static str {
    let ink = ink(id, home, rows);
    if CREW.contains(&ink) { ink } else { "glow" }
}

/// The teammate of the home tree that signs with `name`.
pub fn teammate<'a>(
    name: &str,
    home: &str,
    rows: &'a [SessionSummary],
) -> Option<&'a SessionSummary> {
    teammates(home, rows).find(|row| crate::name::of_session(row) == name)
}
