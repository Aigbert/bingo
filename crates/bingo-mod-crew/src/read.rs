//! The crew as the shared services and the sessions' own journals have it,
//! read for one drawing of the sheet.
use crate::state::{Entry, Mate, Need, Place};
use bingo_collab::mentions::Post;
use bingo_sdk::*;
use bingo_services::{agents_control as agents, rooms_control as rooms};
use std::collections::BTreeSet;

/// What one drawing lists, and the sessions whose changes redraw it.
pub struct Crew {
    pub entries: Vec<Entry>,
    pub watching: BTreeSet<SessionId>,
}

pub async fn read(host: &HostHandle, session: &SessionId) -> Result<Crew, KernelError> {
    let home = home(host, session).await?;
    let mates = mates(host, &home).await?;
    let places = places(host, &home.id, &mates).await?;
    let watching = std::iter::once(home.id.clone())
        .chain(mates.iter().map(|mate| mate.id.clone()))
        .chain(places.iter().map(|place| place.id.clone()))
        .collect();
    Ok(Crew {
        entries: crate::state::ordered(mates, places),
        watching,
    })
}

/// The session the tree hangs from: the crew is the same from anywhere in it.
async fn home(host: &HostHandle, session: &SessionId) -> Result<SessionSummary, KernelError> {
    let mut summary = host.summary(session).await?;
    while let Some(parent) = &summary.parent {
        summary = host.summary(&parent.session).await?;
    }
    Ok(summary)
}

async fn mates(host: &HostHandle, home: &SessionSummary) -> Result<Vec<Mate>, KernelError> {
    let Some(service) = agents::find(host) else {
        return Ok(vec![]);
    };
    let asked = || agents::Session {
        session: home.id.clone(),
    };
    let roster = service.roster(asked()).await.map_err(crate::error)?;
    let team = service.team(asked()).await.map_err(crate::error)?;
    let rows: Vec<SessionSummary> = std::iter::once(home.clone())
        .chain(roster.iter().map(|row| row.summary.clone()))
        .collect();
    let mut mates = Vec::new();
    for row in &roster {
        let summary = &row.summary;
        if bingo_collab::room::Room::of(summary).is_some() {
            continue;
        }
        let mut mate = Mate {
            id: summary.id.clone(),
            name: bingo_collab::name::of_session(summary).into(),
            ink: bingo_collab::face::ink(&summary.id, home.id.as_str(), &rows),
            need: need(summary),
            doing: None,
            since: None,
            role: role(&team, &summary.id),
        };
        if mate.need == Need::Working {
            live(host, &mut mate).await;
        }
        mates.push(mate);
    }
    Ok(mates)
}

fn need(summary: &SessionSummary) -> Need {
    if summary.asking {
        Need::Asking
    } else if summary.busy {
        Need::Working
    } else {
        Need::Idle
    }
}

fn role(team: &[agents::TeamRow], id: &SessionId) -> Option<String> {
    team.iter()
        .find(|row| {
            row.summary
                .as_ref()
                .is_some_and(|summary| &summary.id == id)
        })
        .map(|row| row.role.clone())
}

/// What a working teammate's live turn was asked, and since when. A session
/// that cannot be read says nothing.
async fn live(host: &HostHandle, mate: &mut Mate) {
    let Ok(state) = snapshot(host, &mate.id).await else {
        return;
    };
    let Some(turn) = &state.turn else {
        return;
    };
    mate.since = Some(turn.started_at.as_millisecond());
    mate.doing = state
        .transcript
        .items()
        .iter()
        .filter(|item| item.turn.as_ref() == Some(&turn.id))
        .find_map(|item| Post::of(item))
        .map(|post| post.text);
}

async fn places(
    host: &HostHandle,
    home: &SessionId,
    mates: &[Mate],
) -> Result<Vec<Place>, KernelError> {
    let Some(service) = rooms::find(host) else {
        return Ok(vec![]);
    };
    let rows = service
        .rooms(rooms::Session {
            session: home.clone(),
        })
        .await
        .map_err(crate::error)?;
    let mut places = Vec::new();
    for row in rows {
        let writing = !row.closed && writing(host, &row.id, mates).await;
        places.push(Place {
            id: row.id,
            name: row.name,
            purpose: row.purpose,
            seated: row.seats.len(),
            closed: row.closed,
            writing,
        });
    }
    Ok(places)
}

/// Whether someone the room still waits on is working.
async fn writing(host: &HostHandle, room: &SessionId, mates: &[Mate]) -> bool {
    let Ok(state) = snapshot(host, room).await else {
        return false;
    };
    bingo_collab::owed::of_state(&state)
        .iter()
        .filter_map(|mention| mention.owed_by.chased())
        .any(|name| {
            mates.iter().any(|mate| {
                mate.need == Need::Working && bingo_collab::name::same(&mate.name, name)
            })
        })
}

async fn snapshot(host: &HostHandle, id: &SessionId) -> Result<SessionState, KernelError> {
    let attached = host
        .open(
            SessionSelector::ById { id: id.clone() },
            ClientIdentity {
                name: crate::ID.into(),
                surface: String::new(),
            },
            OpenOptions {
                max_snapshot_bytes: Some(64 * 1024),
                ..Default::default()
            },
        )
        .await?;
    Ok(attached.snapshot)
}
