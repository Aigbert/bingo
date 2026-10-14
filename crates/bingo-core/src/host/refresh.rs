//! Asking each endpoint what it serves, out of everybody's way. A refresh is
//! never on the path of a turn or of `Host::build`: the cached list is what
//! the kernel answers with, and this replaces it when an endpoint gets round
//! to saying otherwise (ADR-0026 §4).
//!
//! One task per provider, each holding a `Weak<Host>`: a host that is shutting
//! down is not held open by an endpoint that has stopped answering, and what
//! comes back for it is dropped.

use super::reach::Reach;
use std::sync::{Arc, Weak};
use std::time::Instant;

use bingo_sdk::{CatalogKind, GatewayEvent, Provider};
use jiff::Timestamp;

use super::Host;
use crate::turn::Endpoint;

/// What one provider's refresh came to: how many ids it answered with, or why
/// it could not be asked.
pub(crate) struct Refreshed {
    pub provider: String,
    pub answer: Result<usize, String>,
}

/// Ask every provider whose list is missing or a day old. Nothing waits for
/// these — not `build`, not a turn, not the shutdown.
pub(super) fn in_background(host: &Arc<Host>) {
    let weak = Arc::downgrade(host);
    tokio::spawn(async move {
        let Some(host) = weak.upgrade() else { return };
        let now = Timestamp::now();
        let stale: Vec<Endpoint> = endpoints(&host)
            .await
            .into_iter()
            .filter(|p| host.served.stale(p.id(), now))
            .collect();
        let weak = Arc::downgrade(&host);
        drop(host);
        for provider in stale {
            let host = weak.clone();
            tokio::spawn(async move { ask(host, provider).await });
        }
    });
}

/// Ask every usable provider now, however fresh its list is, and say what each
/// answered: `/models refresh`, which waits because a person asked it to.
pub(super) async fn now(host: &Arc<Host>) -> Vec<Refreshed> {
    let providers = endpoints(host).await;
    let weak = Arc::downgrade(host);
    futures::future::join_all(
        providers
            .into_iter()
            .map(|provider| ask(weak.clone(), provider)),
    )
    .await
}

/// A provider with no usable credentials is not asked: an endpoint answers a
/// listing with the same 401 it answers a turn with.
fn usable(providers: &[Arc<dyn Provider>]) -> Vec<Arc<dyn Provider>> {
    providers
        .iter()
        .filter(|p| super::auth::signed_in(p.as_ref()))
        .cloned()
        .collect()
}

/// One provider's list, kept if it came and announced if it changed. A failure
/// keeps whatever was cached: an endpoint that could not be reached has not
/// withdrawn its models.
async fn ask(host: Weak<Host>, provider: Endpoint) -> Refreshed {
    let started = Instant::now();
    let answer = provider
        .models()
        .await
        .map_err(|error| format!("model list failed ({:?})", error.code()));
    if let Some(host) = host.upgrade() {
        let _edit = host.registry.provider_state.instances.edit.lock().await;
        if !current(&host, &provider).await {
            return Refreshed {
                provider: provider.id().into(),
                answer: answer.map(|models| models.len()),
            };
        }
        let at = Timestamp::now();
        let reach = match &answer {
            Ok(_) => Reach::Ok {
                ms: started.elapsed().as_millis() as u64,
                at,
            },
            Err(error) => Reach::Down {
                error: error.clone(),
                at,
            },
        };
        host.registry
            .provider_state
            .reach
            .record(provider.id(), reach);
        let _ = host.gateway.send(GatewayEvent::CatalogChanged {
            kind: CatalogKind::Providers,
        });
        if let Ok(models) = &answer
            && host
                .served
                .record(provider.id(), models.clone(), Timestamp::now())
        {
            let _ = host.gateway.send(GatewayEvent::CatalogChanged {
                kind: CatalogKind::Models,
            });
        }
    }
    Refreshed {
        provider: provider.id().into(),
        answer: answer.map(|models| models.len()),
    }
}

async fn current(host: &Host, endpoint: &Endpoint) -> bool {
    match endpoint {
        Endpoint::Provider(provider) => host
            .providers()
            .await
            .iter()
            .any(|p| Arc::ptr_eq(p, provider)),
        Endpoint::Agent(_) => true,
    }
}

pub(super) async fn one(host: &Arc<Host>, provider: Arc<dyn Provider>) -> Refreshed {
    ask(Arc::downgrade(host), Endpoint::Provider(provider)).await
}

async fn endpoints(host: &Host) -> Vec<Endpoint> {
    usable(&host.providers().await)
        .into_iter()
        .map(Endpoint::Provider)
        .chain(host.registry.agents.iter().cloned().map(Endpoint::Agent))
        .collect()
}
