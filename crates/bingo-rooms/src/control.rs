use crate::{door, name, room, seat};
use bingo_sdk::*;
use bingo_services::rooms_control::*;

#[async_trait::async_trait]
impl RoomsControl for crate::RoomWriter {
    async fn rooms(&self, request: Session) -> Result<Vec<RoomRow>, ServiceError> {
        rows(self.control_host()?, &request.session)
            .await
            .map_err(failed)
    }
    async fn seat(&self, request: Seat) -> Result<String, ServiceError> {
        change(
            self.control_host()?,
            &request.session,
            "seat",
            &request.name,
            &request.agent,
        )
        .await
        .map_err(failed)
    }
    async fn unseat(&self, request: Seat) -> Result<String, ServiceError> {
        change(
            self.control_host()?,
            &request.session,
            "unseat",
            &request.name,
            &request.agent,
        )
        .await
        .map_err(failed)
    }
    async fn set_purpose(&self, request: Purpose) -> Result<String, ServiceError> {
        change(
            self.control_host()?,
            &request.session,
            "purpose",
            &request.name,
            &request.text,
        )
        .await
        .map_err(failed)
    }
    async fn close(&self, request: Named) -> Result<String, ServiceError> {
        change(
            self.control_host()?,
            &request.session,
            "close",
            &request.name,
            "",
        )
        .await
        .map_err(failed)
    }
}
fn failed(error: KernelError) -> ServiceError {
    ServiceError(error.message)
}
pub async fn rows(host: &HostHandle, session: &SessionId) -> Result<Vec<RoomRow>, KernelError> {
    let mut rows = vec![];
    for (id, room) in room::under(host, session).await? {
        let state = room::read(host, &id).await;
        rows.push(RoomRow {
            id: id.clone(),
            name: room.title,
            purpose: state.as_ref().and_then(room::purpose_of),
            seats: state
                .as_ref()
                .map(room::roster_of)
                .unwrap_or_default()
                .iter()
                .map(crate::ear::Seat::said)
                .collect(),
            holder: name::signed_by(&host.summary(&room.parent).await?),
            closed: state.as_ref().is_some_and(room::closed_of),
        });
    }
    Ok(rows)
}
pub async fn change(
    host: &HostHandle,
    session: &SessionId,
    verb: &str,
    name: &str,
    text: &str,
) -> Result<String, KernelError> {
    let caller = door::own(host, session).await?;
    let (id, room) = door::entered(host, &caller, name).await?;
    let by = crate::name::signed_by(&caller);
    match verb {
        "seat" => {
            seat::join(host, &id, &room, &[crate::ear::Seat::read(text)?], &by).await?;
        }
        "unseat" => {
            seat::leave(host, &id, &room, &[text.into()], &by).await?;
        }
        "close" => {
            seat::close(host, &id, &room, &by, None).await?;
            return Ok(format!("{}: closed", room.title));
        }
        "purpose" => purpose(host, &id, &room, text).await?,
        _ => {
            return Err(KernelError::new(
                ErrorCode::InvalidInput,
                "unknown room verb",
            ));
        }
    }
    Ok(format!("{}: {verb} {text}", room.title))
}
async fn purpose(
    host: &HostHandle,
    id: &SessionId,
    room: &room::Room,
    text: &str,
) -> Result<(), KernelError> {
    if room.closed || text.trim().is_empty() {
        return Err(KernelError::new(
            ErrorCode::InvalidInput,
            "a standing room needs a nonempty purpose",
        ));
    }
    let state = room::read(host, id)
        .await
        .ok_or_else(|| KernelError::new(ErrorCode::SessionNotFound, "room is unavailable"))?;
    let mut opened = room::Opened::of_state(&state).unwrap_or(room::Opened {
        purpose: None,
        by: name::PARENT.into(),
    });
    opened.purpose = Some(text.into());
    host.extend(id, crate::PLUGIN, room::OPENED, opened.payload())
        .await
}
