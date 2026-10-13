//! Reverse requests negotiate capabilities before shared host operations.

use crate::codec::{INTERNAL_ERROR, METHOD_NOT_FOUND, RpcError};
use crate::service::ServiceCalls;
use crate::wire::{Cap, name};
pub use bingo_plugin_host::host::{
    Complete, Completed, Deliver, Extend, Place, StateGet, StateSet, StateValue,
};
use bingo_sdk::HostHandle;
use serde_json::Value;
use std::collections::BTreeSet;
use std::sync::Arc;

pub struct Server {
    pub(crate) contexts: crate::context::Chains,
    plugin: String,
    host: HostHandle,
    service: Arc<dyn ServiceCalls>,
    state: crate::state::Store,
    capabilities: tokio::sync::watch::Sender<Option<BTreeSet<Cap>>>,
}

impl Server {
    pub fn new(
        plugin: &str,
        host: HostHandle,
        service: Arc<dyn ServiceCalls>,
        state: crate::state::Store,
    ) -> Self {
        Self {
            contexts: Default::default(),
            plugin: plugin.into(),
            host,
            service,
            state,
            capabilities: tokio::sync::watch::channel(None).0,
        }
    }

    pub fn initialize(&self, plugin: &BTreeSet<Cap>) {
        self.capabilities.send_replace(Some(crate::wire::negotiated(
            &crate::wire::capabilities(),
            plugin,
        )));
    }

    pub fn permits(&self, method: &str) -> Result<(), RpcError> {
        let catalogue = catalogue_capability(method);
        if !crate::wire::capabilities().contains(method) && catalogue.is_none() {
            return Err(unknown(method));
        }
        if self.capabilities.borrow().as_ref().is_some_and(|caps| {
            caps.contains(method) || catalogue.is_some_and(|cap| caps.contains(cap))
        }) {
            return Ok(());
        }
        Err(RpcError::new(
            METHOD_NOT_FOUND,
            format!("{} did not negotiate capability {method}", self.plugin),
        ))
    }

    pub async fn answer(&self, method: &str, mut params: Value) -> Result<Value, RpcError> {
        let context = params
            .as_object_mut()
            .and_then(|params| params.remove("_bingo"));
        let context = context
            .map(serde_json::from_value)
            .transpose()
            .map_err(failed)?
            .unwrap_or_default();
        let reads = self.contexts.reads(&context);
        let rendering = self.contexts.rendering(&context);
        let asked = self.contexts.asked(&context);
        let calling = self.contexts.bind(&self.plugin, context).map_err(failed)?;
        reads
            .scope(bingo_plugin_host::invocation::scope(
                calling,
                bingo_sdk::render::under_input(
                    asked,
                    bingo_plugin_host::state::rendering(
                        rendering,
                        self.answer_bound(method, params),
                    ),
                ),
            ))
            .await
    }

    pub(crate) async fn subscribe(
        &self,
        session: bingo_sdk::SessionId,
        request: String,
        reads: Vec<bingo_plugin_host::state::Read>,
    ) -> Result<(), RpcError> {
        self.state
            .subscribe(&self.host, session, request, reads)
            .await
    }

    async fn answer_bound(&self, method: &str, params: Value) -> Result<Value, RpcError> {
        if !crate::wire::capabilities().contains(method) {
            return Err(unknown(method));
        }
        let mut capabilities = self.capabilities.subscribe();
        capabilities
            .wait_for(|caps| caps.is_some())
            .await
            .map_err(failed)?;
        self.permits(method)?;
        if method == name::SERVICE_CALL {
            return bingo_plugin_host::ops::scope(
                &self.host,
                &self.plugin,
                &self.state,
                self.service.call(params),
            )
            .await;
        }
        let (noun, _) = bingo_plugin_host::ops::admit(&self.host, &self.plugin, method, &params)
            .await
            .map_err(failed)?;
        if self
            .host
            .service_wire(&format!("bingo.plugin.{noun}"))
            .is_some()
        {
            return bingo_plugin_host::ops::call(
                &self.host,
                &self.plugin,
                &self.state,
                method,
                params,
            )
            .await
            .map_err(failed);
        }
        bingo_plugin_host::host::Operations {
            plugin: &self.plugin,
            host: &self.host,
            state: &self.state,
        }
        .answer(method, params)
        .await
    }
}

pub(crate) fn unknown(method: &str) -> RpcError {
    RpcError::new(
        METHOD_NOT_FOUND,
        format!("the host does not serve {method}"),
    )
}
fn failed(error: impl std::fmt::Display) -> RpcError {
    RpcError::new(INTERNAL_ERROR, error.to_string())
}

fn catalogue_capability(method: &str) -> Option<&'static str> {
    let event = method.strip_prefix("intercept/")?;
    let (event, capability) = match event.strip_suffix("/after") {
        Some(event) => (event, "intercept/*/after"),
        None => (event, "intercept/*"),
    };
    (event.contains('.')
        && !event.contains(['*', '!'])
        && bingo_sdk::catalogue::Pattern::parse(event).is_ok())
    .then_some(capability)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::{Catalog, CatalogEntry, CatalogKind};
    use serde_json::json;

    struct NoServices;
    #[async_trait::async_trait]
    impl ServiceCalls for NoServices {
        async fn call(&self, _: Value) -> Result<Value, RpcError> {
            Err(unknown("service/call"))
        }
    }

    #[tokio::test]
    async fn model_completion_requires_the_catalogues_granted_noun() {
        let host = bingo_sdk::testing::TestHost::default();
        host.on_catalog(|kind| async move {
            assert_eq!(kind, CatalogKind::Plugins);
            Ok(Catalog {
                kind,
                entries: vec![CatalogEntry {
                    id: "stub".into(),
                    label: "stub".into(),
                    meta: json!({"enabled":true, "tier":"rewrite", "uses":[]}),
                }],
            })
        });
        let home = tempfile::tempdir().unwrap();
        let server = Server::new(
            "stub",
            host.handle(),
            Arc::new(NoServices),
            crate::state::Store::new(home.path(), "stub"),
        );
        server.initialize(&crate::wire::capabilities());
        let error = server
            .answer(
                name::MODEL_COMPLETE,
                json!({"session":"ses_1", "request":{
                    "model":"fake", "maxTokens":1, "system":[], "messages":[], "tools":[]
                }}),
            )
            .await
            .unwrap_err();
        assert!(error.message.contains("model grant"), "{}", error.message);
    }

    #[tokio::test]
    async fn host_operations_require_the_catalogues_granted_nouns() {
        let host = bingo_sdk::testing::TestHost::default();
        host.on_catalog(|kind| async move {
            Ok(Catalog {
                kind,
                entries: vec![CatalogEntry {
                    id: "stub".into(),
                    label: "stub".into(),
                    meta: json!({"enabled":true, "tier":"rewrite", "uses":[]}),
                }],
            })
        });
        let home = tempfile::tempdir().unwrap();
        let server = Server::new(
            "stub",
            host.handle(),
            Arc::new(NoServices),
            crate::state::Store::new(home.path(), "stub"),
        );
        server.initialize(&crate::wire::capabilities());
        for (method, params, noun) in [
            (
                "host/state/get",
                json!({"scope":"ses_1", "key":"key"}),
                "state",
            ),
            (
                "host/state/set",
                json!({"scope":"global", "key":"key", "value":1}),
                "store",
            ),
            ("host/place", json!({"slot":"calls", "view":null}), "ui"),
            (
                "host/extend",
                json!({"session":"ses_1", "key":"key", "value":1}),
                "session",
            ),
            ("host/deliver", json!({}), "session"),
        ] {
            let error = server.answer(method, params).await.expect_err(method);
            assert!(
                error.message.contains(&format!("{noun} grant")),
                "{method}: {}",
                error.message
            );
        }
    }
}
