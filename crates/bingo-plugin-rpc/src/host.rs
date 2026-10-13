//! Reverse requests negotiate capabilities before shared host operations.

pub use bingo_plugin_host::host::{Place, Extend, Deliver, StateGet, StateSet, StateValue, Complete, Completed};
use serde_json::Value;
use crate::codec::{INTERNAL_ERROR, METHOD_NOT_FOUND, RpcError};
use crate::service::ServiceCalls;
use crate::wire::{Cap, name};
use bingo_sdk::HostHandle;
use std::collections::BTreeSet;
use std::sync::Arc;

pub struct Server {
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

    pub async fn answer(&self, method: &str, params: Value) -> Result<Value, RpcError> {
        if !crate::wire::capabilities().contains(method) {
            return Err(unknown(method));
        }
        let mut capabilities = self.capabilities.subscribe();
        capabilities
            .wait_for(|caps| caps.is_some())
            .await
            .map_err(failed)?;
        self.permits(method)?;
        if method == name::SERVICE_CALL { return self.service.call(params).await; }
        bingo_plugin_host::host::Operations { plugin: &self.plugin, host: &self.host, state: &self.state }.answer(method, params).await
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
