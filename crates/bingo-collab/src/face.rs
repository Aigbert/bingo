//! A teammate's ink follows creation order in the home tree, including stored rows.
use bingo_sdk::{SessionId, SessionSummary};

const CREW: [&str; 6] = ["crew1", "crew2", "crew0", "crew3", "crew4", "crew5"];

pub fn ink(id: &SessionId, home: &str, rows: &[SessionSummary]) -> &'static str {
    let Some(row) = rows.iter().find(|row| &row.id == id) else {
        return "muted";
    };
    if crate::room::Room::of(row).is_some() {
        return "plan";
    }
    if row.id.as_str() == home {
        return "glow";
    }
    let mut teammates = rows
        .iter()
        .filter(|row| {
            row.parent.as_ref().map(|parent| parent.session.as_str()) == Some(home)
                && crate::room::Room::of(row).is_none()
        })
        .collect::<Vec<_>>();
    teammates.sort_by(|a, b| {
        a.created_at
            .cmp(&b.created_at)
            .then_with(|| a.id.cmp(&b.id))
    });
    teammates
        .iter()
        .position(|row| &row.id == id)
        .map_or("muted", |slot| CREW[slot % CREW.len()])
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

/// The teammate of the home tree that signs with `name`: a session the home
/// started that is not a room.
pub fn teammate<'a>(
    name: &str,
    home: &str,
    rows: &'a [SessionSummary],
) -> Option<&'a SessionSummary> {
    rows.iter().find(|row| {
        crate::name::of_session(row) == name
            && row.parent.as_ref().map(|parent| parent.session.as_str()) == Some(home)
            && crate::room::Room::of(row).is_none()
    })
}
