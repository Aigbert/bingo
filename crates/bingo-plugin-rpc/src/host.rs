//! Reverse requests translate wire envelopes onto the host's own doors.

use bingo_sdk::slot::Slot;
use bingo_sdk::{Input, ModelEvent, ModelRequest, SessionId, TurnId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Place {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionId>,
    /// The plugin's placement key. Several independent slots can coexist.
    pub slot: String,
    pub view: Option<Slot>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Extend {
    pub session: SessionId,
    pub key: String,
    pub value: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Deliver {
    pub session: SessionId,
    pub input: Input,
    pub wake: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StateGet {
    pub scope: String,
    pub key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StateSet {
    pub scope: String,
    pub key: String,
    pub value: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub if_version: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StateValue {
    pub version: u64,
    pub value: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Complete {
    pub session: SessionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn: Option<TurnId>,
    pub request: ModelRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Completed {
    pub events: Vec<ModelEvent>,
}

use crate::codec::{INTERNAL_ERROR, INVALID_PARAMS, METHOD_NOT_FOUND, RpcError};
use crate::service::ServiceCalls;
use crate::wire::{Cap, name};
use bingo_sdk::{Delivery, HostHandle, IntentId, Origin, SessionFilter};
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
        if !crate::wire::capabilities().contains(method) {
            return Err(unknown(method));
        }
        if self
            .capabilities
            .borrow()
            .as_ref()
            .is_some_and(|caps| caps.contains(method))
        {
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
        match method {
            name::SERVICE_CALL => self.service.call(params).await,
            name::HOST_PLACE => self.place(parse(params)?).await,
            name::HOST_EXTEND => self.extend(parse(params)?).await,
            name::HOST_DELIVER => self.deliver(parse(params)?).await,
            name::STATE_GET => self.get(parse(params)?).await,
            name::STATE_SET => self.set(parse(params)?).await,
            name::MODEL_COMPLETE => self.complete(parse(params)?).await,
            _ => Err(unknown(method)),
        }
    }

    async fn complete(&self, params: Complete) -> Result<Value, RpcError> {
        let events = self
            .host
            .complete(
                &params.session,
                params.turn.as_ref(),
                bingo_sdk::Completion {
                    plugin: self.plugin.clone(),
                    request: params.request,
                },
            )
            .await
            .map_err(failed)?;
        serde_json::to_value(Completed { events }).map_err(failed)
    }

    async fn get(&self, params: StateGet) -> Result<Value, RpcError> {
        serde_json::to_value(self.state.get(params.scope, params.key).await?).map_err(failed)
    }
    async fn set(&self, params: StateSet) -> Result<Value, RpcError> {
        serde_json::to_value(
            self.state
                .set(params.scope, params.key, params.value, params.if_version)
                .await?,
        )
        .map_err(failed)
    }

    async fn place(&self, params: Place) -> Result<Value, RpcError> {
        let payload = match params.view {
            Some(view) => bingo_sdk::slot::with(serde_json::json!({}), [view]),
            None => Value::Null,
        };
        let sessions = match params.session {
            Some(session) => vec![session],
            None => self
                .host
                .sessions(SessionFilter::default())
                .await
                .map_err(failed)?
                .into_iter()
                .map(|summary| summary.id)
                .collect(),
        };
        for session in sessions {
            self.host
                .signal(&session, &self.plugin, &params.slot, payload.clone())
                .await
                .map_err(failed)?;
        }
        Ok(Value::Null)
    }

    async fn extend(&self, params: Extend) -> Result<Value, RpcError> {
        self.host
            .extend(&params.session, &self.plugin, &params.key, params.value)
            .await
            .map_err(failed)?;
        Ok(Value::Null)
    }

    async fn deliver(&self, mut params: Deliver) -> Result<Value, RpcError> {
        let Input::Text { origin, .. } = &mut params.input else {
            return Err(RpcError::new(
                INVALID_PARAMS,
                "host/deliver requires text input",
            ));
        };
        *origin = Origin::surface(format!("plugin:{}", self.plugin));
        let delivery = if params.wake {
            Delivery::Wake
        } else {
            Delivery::Hold
        };
        self.host
            .deliver(&params.session, IntentId::mint(), params.input, delivery)
            .await
            .map_err(failed)?;
        Ok(Value::Null)
    }
}

pub(crate) fn unknown(method: &str) -> RpcError {
    RpcError::new(
        METHOD_NOT_FOUND,
        format!("the host does not serve {method}"),
    )
}
fn parse<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, RpcError> {
    serde_json::from_value(params).map_err(|error| RpcError::new(INVALID_PARAMS, error.to_string()))
}
fn failed(error: impl std::fmt::Display) -> RpcError {
    RpcError::new(INTERNAL_ERROR, error.to_string())
}
