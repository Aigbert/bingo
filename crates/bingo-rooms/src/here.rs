//! `/seat`, `/unseat` and `/close`: the room's own verbs, typed in the room on
//! screen (`seatAgent`, `unseatAgent`, `closeRoom`). The caller is the session
//! the room hangs under, so whoever reads the room may change it.
//!
//! A word missing or a name that does not fit is answered, not failed: the
//! line is written back with what it needs said beside it.

use async_trait::async_trait;
use bingo_sdk::toast::Say;
use bingo_sdk::{
    Action, ArgSpec, Command, CommandContext, CommandOutcome, CommandSpec, Driver, ErrorCode,
    HostHandle, KernelError, SessionId, SessionState,
};
use serde_json::json;

use crate::ear::Seat;
use crate::room::{self, Room};
use crate::{door, name, seat};

/// The family a palette lists only in an open room.
pub const FAMILY: &str = "rooms/here";

/// Which of the three verbs a command is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Seat,
    Unseat,
    Close,
}

/// One verb as a command of its own.
#[derive(Clone, Copy, Debug)]
pub struct HereCommand(pub Verb);

impl Verb {
    fn name(self) -> &'static str {
        match self {
            Verb::Seat => "seat",
            Verb::Unseat => "unseat",
            Verb::Close => name::CLOSE,
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Verb::Seat => "bring an agent into this room",
            Verb::Unseat => "ask an agent to leave this room",
            Verb::Close => "close this room and say why",
        }
    }
}

#[async_trait]
impl Command for HereCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            opens: false,
            keys: vec![],
            hidden: false,
            name: self.0.name().into(),
            aliases: vec![],
            hint: self.0.hint().into(),
            args: ArgSpec::Free {
                hint: match self.0 {
                    Verb::Close => "why".into(),
                    Verb::Seat | Verb::Unseat => "name".into(),
                },
            },
            instant: true,
            family: FAMILY.into(),
        }
    }

    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let (id, room, state) = viewed(&cx.host, &cx.session).await?;
        let by = name::signed_by(&door::own(&cx.host, &room.parent).await?);
        let said = args.trim();
        match self.0 {
            Verb::Seat => seating(&cx.host, (&id, &room, &state), said, &by).await,
            Verb::Unseat => unseating(&cx.host, (&id, &room, &state), said, &by).await,
            Verb::Close => closing(&cx.host, (&id, &room), said, &by).await,
        }
    }
}

/// The room on screen, as its journal has it, while it still stands.
async fn viewed(
    host: &HostHandle,
    session: &SessionId,
) -> Result<(SessionId, Room, SessionState), KernelError> {
    let summary = door::own(host, session).await?;
    let outside = || KernelError::new(ErrorCode::InvalidInput, "this works inside a room");
    if summary.driver != Driver::Log {
        return Err(outside());
    }
    let room = Room::of(&summary).ok_or_else(outside)?;
    let state = room::read(host, session).await.ok_or_else(|| {
        KernelError::new(
            ErrorCode::SessionNotFound,
            format!("{} could not be read", room.title),
        )
    })?;
    if room::closed_of(&state) {
        return Err(KernelError::new(
            ErrorCode::InvalidInput,
            "this room is closed",
        ));
    }
    Ok((summary.id, room.seated(&state), state))
}

type Here<'a> = (&'a SessionId, &'a Room, &'a SessionState);

async fn seating(
    host: &HostHandle,
    (id, room, _): Here<'_>,
    asked: &str,
    by: &str,
) -> Result<CommandOutcome, KernelError> {
    let free = free(host, room).await?;
    if asked.is_empty() {
        let names = match free.is_empty() {
            true => "everyone is here".to_string(),
            false => free.join(", "),
        };
        return Ok(rewrite(
            "/seat ",
            Say::new(["seat who?".to_string(), names]),
        ));
    }
    let wanted = Seat::read(asked)?;
    if seated(room, &wanted.name) {
        return Ok(told(format!("{} is already here", wanted.name)));
    }
    if !free.iter().any(|held| name::same(held, &wanted.name)) {
        return Ok(told(format!("no agent called {}", wanted.name)));
    }
    seat::join(host, id, room, &[wanted], by).await?;
    Ok(CommandOutcome::Applied { message: None })
}

async fn unseating(
    host: &HostHandle,
    (id, room, _): Here<'_>,
    asked: &str,
    by: &str,
) -> Result<CommandOutcome, KernelError> {
    if asked.is_empty() {
        let names = room.members.join(", ");
        return Ok(rewrite(
            "/unseat ",
            Say::new(["unseat who?".to_string(), names]),
        ));
    }
    if !seated(room, asked) {
        return Ok(told(format!("{asked} is not in this room")));
    }
    seat::leave(host, id, room, &[asked.to_string()], by).await?;
    Ok(CommandOutcome::Applied { message: None })
}

async fn closing(
    host: &HostHandle,
    (id, room): (&SessionId, &Room),
    why: &str,
    by: &str,
) -> Result<CommandOutcome, KernelError> {
    if why.is_empty() {
        return Ok(rewrite("/close ", Say::new(["say why the room closes"])));
    }
    seat::close(host, id, room, by, Some(why)).await?;
    Ok(CommandOutcome::Applied { message: None })
}

/// The teammates beside the room no seat holds yet.
async fn free(host: &HostHandle, room: &Room) -> Result<Vec<String>, KernelError> {
    let siblings = crate::post::siblings_of(host, room).await?;
    Ok(siblings
        .iter()
        .filter(|summary| summary.driver == Driver::Model)
        .filter_map(|summary| summary.title.clone())
        .filter(|title| !seated(room, title))
        .collect())
}

fn seated(room: &Room, who: &str) -> bool {
    room.members.iter().any(|held| name::same(held, who))
}

/// The line written back for the person to finish, with what it needs.
fn rewrite(line: &str, said: Say) -> CommandOutcome {
    CommandOutcome::Surface {
        action: Action {
            name: "Composer".into(),
            args: json!({ "text": line, "say": said }),
        },
        fallback: Some(said.plain()),
    }
}

fn told(text: String) -> CommandOutcome {
    let said = Say::new([text.clone()]);
    said.outcome(text)
}

#[cfg(test)]
mod tests;
