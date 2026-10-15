//! The live status card, drawn through the shared Status site.
mod command;
mod dispatch;
pub mod draw;
mod guide;
mod keys;
mod live;
pub mod rows;
use bingo_sdk::*;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Instant,
};

const ID: &str = "bingo.mod.status";
const REQUEST: &str = "status";
type Selections = Arc<Mutex<BTreeMap<SessionId, Selection>>>;
#[derive(Clone)]
struct Selection {
    label: String,
    seen: Instant,
}
impl From<String> for Selection {
    fn from(label: String) -> Self {
        Self {
            label,
            seen: Instant::now(),
        }
    }
}
impl From<&str> for Selection {
    fn from(label: &str) -> Self {
        label.to_string().into()
    }
}

pub struct Status {
    selections: Selections,
    started: Instant,
    stop: CancellationToken,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            selections: Default::default(),
            started: Instant::now(),
            stop: Default::default(),
        }
    }
}
static MANIFEST: PluginManifest = PluginManifest {
    id: ID,
    version: env!("CARGO_PKG_VERSION"),
    uses: &[bingo_services::mcp::CAPABILITY],
    requires: &[],
    provides: &[
        "command:bingo.mod.status:ui",
        "service:bingo.mod.status.pages",
    ],
    config: None,
};
#[async_trait::async_trait]
impl Plugin for Status {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }
    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        registrar.add(Contribution::Command(Arc::new(keys::Handler(
            self.selections.clone(),
        ))));
        let links = vec![
            Arc::new(command::Open(self.selections.clone())) as Arc<dyn Interceptor>,
            Arc::new(draw::Drawing {
                selections: self.selections.clone(),
                started: self.started,
            }),
        ];
        for link in links {
            registrar.add(Contribution::Interceptor {
                placement: plugin::Placement::Builtin,
                external: None,
                link,
            });
        }
        registrar.add(Contribution::Hook(Arc::new(live::Refresh(
            self.selections.clone(),
        ))));
        registrar.add(guide::contribution(registrar));
        Ok(())
    }
    async fn start(&self, host: HostHandle) -> Result<(), PluginError> {
        live::start(host, self.selections.clone(), self.stop.clone());
        Ok(())
    }
    async fn stop(&self) -> Result<(), PluginError> {
        self.stop.cancel();
        Ok(())
    }
}
fn error(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::Internal, error.to_string())
}
async fn snapshot(host: &HostHandle, session: &SessionId) -> Result<Attachment, KernelError> {
    host.open(
        SessionSelector::ById {
            id: session.clone(),
        },
        ClientIdentity {
            name: ID.into(),
            surface: "terminal".into(),
        },
        OpenOptions {
            max_snapshot_bytes: Some(64 * 1024),
            ..Default::default()
        },
    )
    .await
}

#[cfg(test)]
mod keys_tests;
#[cfg(test)]
mod tests;
