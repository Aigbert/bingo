//! Persist the transcript's model view without adding it to the client wire.

use crate::{SessionState, transcript::checkpoint::Stored};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize)]
struct State {
    session: SessionState,
    transcript: Stored,
}

pub(super) fn serialize<S: Serializer>(
    state: &SessionState,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut session = state.clone();
    let transcript = std::mem::take(&mut session.transcript);
    State {
        session,
        transcript: Stored::from(&transcript),
    }
    .serialize(serializer)
}

pub(super) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<SessionState, D::Error> {
    let State {
        mut session,
        transcript,
    } = State::deserialize(deserializer)?;
    if session.transcript != crate::Transcript::new() {
        return Err(serde::de::Error::custom(
            "checkpoint session contains a second transcript",
        ));
    }
    session.transcript = transcript.restore().map_err(serde::de::Error::custom)?;
    Ok(session)
}
