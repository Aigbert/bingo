//! Both service faces share the rooms writer and its host.
use crate::{post, room, serial};
use async_trait::async_trait;
use bingo_sdk::{HostHandle, ServiceError};
use bingo_services::room_post::{Posted, Request, RoomPost, Said};
use std::sync::OnceLock;

#[derive(Debug, Default)]
pub struct RoomWriter {
    host: OnceLock<HostHandle>,
    serial: tokio::sync::Mutex<()>,
}

impl RoomWriter {
    pub(crate) fn control_host(&self) -> Result<&HostHandle, ServiceError> {
        self.host
            .get()
            .ok_or_else(|| ServiceError("rooms have not started".into()))
    }
    pub fn new(host: HostHandle) -> Self {
        Self {
            host: OnceLock::from(host),
            serial: tokio::sync::Mutex::new(()),
        }
    }

    pub(crate) fn start(&self, host: HostHandle) {
        let _ = self.host.set(host);
    }

    async fn weighed(&self, host: &HostHandle, request: &Request) -> Result<Posted, ServiceError> {
        let there = room::read(host, &request.room).await;
        let here = room::read(host, &request.caller).await;
        let text = match (there, here) {
            (Some(there), Some(here)) => {
                let title = there
                    .summary
                    .title
                    .as_deref()
                    .unwrap_or(request.room.as_str());
                if room::closed_of(&there) {
                    return Ok(Posted::Bounced(format!(
                        "{title} is closed; nothing lands there — open another room"
                    )));
                }
                let text = match &request.said {
                    Said::Text(text) => text.clone(),
                    Said::Again => match serial::draft(&here, &request.cut, title) {
                        Some(text) => text,
                        None => return Ok(nothing_to_repeat(title)),
                    },
                };
                if let Some(bounce) =
                    serial::bounce(&there, &here, &request.cut, title, &request.author)
                {
                    return Ok(Posted::Bounced(
                        bounce
                            .parts
                            .iter()
                            .filter_map(bingo_sdk::ContentPart::as_text)
                            .collect(),
                    ));
                }
                text
            }
            _ => match &request.said {
                Said::Text(text) => text.clone(),
                Said::Again => return Ok(nothing_to_repeat(request.room.as_str())),
            },
        };
        post::write(host, &request.room, &request.author, text.clone())
            .await
            .map_err(|error| ServiceError(error.message))?;
        Ok(Posted::Landed(text))
    }
}

fn nothing_to_repeat(title: &str) -> Posted {
    Posted::Bounced(format!(
        "nothing to post again in {title}: no post of yours bounced there"
    ))
}

#[async_trait]
impl RoomPost for RoomWriter {
    async fn post(&self, request: Request) -> Result<Posted, ServiceError> {
        let host = self
            .host
            .get()
            .ok_or_else(|| ServiceError("rooms have not started".into()))?;
        let _serial = self.serial.lock().await;
        self.weighed(host, &request).await
    }
}
