//! Transport-neutral host operations and their existing wire envelopes.
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

pub use crate::state::StateValue;

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

use bingo_jsonrpc::codec::{INVALID_PARAMS, METHOD_NOT_FOUND, RpcError};
use bingo_sdk::{Delivery, HostHandle, IntentId, Origin, SessionFilter};
const INTERNAL_ERROR: i64 = -32603;

pub struct Operations<'a> {
    pub plugin: &'a str,
    pub host: &'a HostHandle,
    pub state: &'a crate::state::Store,
}
impl Operations<'_> {
    pub async fn answer(&self, method: &str, params: Value) -> Result<Value, RpcError> {
        match method {
            "host/ui/invalidate" => self.invalidate(parse(params)?).await,
            "host/place" => self.place(parse(params)?).await,
            "host/extend" => self.extend(parse(params)?).await,
            "host/deliver" => self.deliver(parse(params)?).await,
            "host/state/get" => self.get(parse(params)?).await,
            "host/state/set" => self.set(parse(params)?).await,
            "host/model/complete" => self.complete(parse(params)?).await,
            _ => Err(RpcError::new(
                METHOD_NOT_FOUND,
                format!("the host does not serve {method}"),
            )),
        }
    }
    async fn invalidate(&self, params: bingo_sdk::render::Invalidate) -> Result<Value, RpcError> {
        self.host
            .invalidate(&params.session, params.requests)
            .await
            .map_err(failed)?;
        Ok(Value::Null)
    }

    async fn complete(&self, params: Complete) -> Result<Value, RpcError> {
        let events = self
            .host
            .complete(
                &params.session,
                params.turn.as_ref(),
                bingo_sdk::Completion {
                    plugin: self.plugin.to_string(),
                    request: params.request,
                },
            )
            .await
            .map_err(failed)?;
        serde_json::to_value(Completed { events }).map_err(failed)
    }

    async fn get(&self, params: StateGet) -> Result<Value, RpcError> {
        let value = self
            .state
            .get(params.scope.clone(), params.key.clone())
            .await?;
        crate::state::ReadLog::record(crate::state::Read {
            plugin: self.plugin.into(),
            scope: params.scope,
            key: params.key,
            version: value.as_ref().map(|value| value.version),
        });
        serde_json::to_value(value).map_err(failed)
    }
    async fn set(&self, params: StateSet) -> Result<Value, RpcError> {
        let value = self
            .state
            .set(
                params.scope.clone(),
                params.key.clone(),
                params.value,
                params.if_version,
            )
            .await?;
        self.state
            .changed(self.host, &params.scope, &params.key)
            .await?;
        serde_json::to_value(value).map_err(failed)
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
                .signal(&session, self.plugin, &params.slot, payload.clone())
                .await
                .map_err(failed)?;
        }
        Ok(Value::Null)
    }

    async fn extend(&self, params: Extend) -> Result<Value, RpcError> {
        self.host
            .extend(&params.session, self.plugin, &params.key, params.value)
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
fn parse<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, RpcError> {
    serde_json::from_value(params).map_err(|error| RpcError::new(INVALID_PARAMS, error.to_string()))
}
fn failed(error: impl std::fmt::Display) -> RpcError {
    RpcError::new(INTERNAL_ERROR, error.to_string())
}

/// Wire faces report conflicts in words while the protocol door keeps its data.
pub fn error_message(error: RpcError) -> String {
    match error.data {
        Some(data) if data.get("version").is_some() => {
            format!("{}; held version: {}", error.message, data["version"])
        }
        _ => error.message,
    }
}

/// Both runtime commands enter the callback registry's current generation.
pub async fn handler(
    plugin: &str,
    args: &str,
    cx: &bingo_sdk::CommandContext,
) -> Result<bingo_sdk::CommandOutcome, bingo_sdk::KernelError> {
    use bingo_sdk::*;
    let input: Value = serde_json::from_str(args)
        .map_err(|error| KernelError::new(ErrorCode::InvalidInput, error.to_string()))?;
    let registry = cx
        .host
        .service(&crate::callback::KEY)
        .ok_or_else(|| KernelError::new(ErrorCode::Internal, "callback registry is unavailable"))?;
    let context = HookContext {
        session: cx.session.clone(),
        turn: None,
        item: None,
        cwd: cx.cwd.clone(),
        provider: None,
        model: None,
        host: cx.host.clone(),
    };
    let result = tokio::select! {
        biased;
        _ = cx.cancel.cancelled() => return Ok(CommandOutcome::Applied {message:None}),
        result = async {
            let executor = registry.prepared(plugin).await?;
            executor.call("__ui__", input, &context).await
        } => result,
    };
    let notice = match result {
        Ok(value) if value["stale"].as_bool() == Some(true) => {
            Some("This control belongs to an earlier drawing; try the current one.".to_string())
        }
        Ok(value) if value.get("handled").and_then(Value::as_bool).is_some() => {
            return Ok(CommandOutcome::Handled {
                handled: value["handled"] != false,
            });
        }
        Ok(_) => None,
        Err(error) => Some(error),
    };
    Ok(match notice {
        Some(text) => CommandOutcome::Record {
            body: ItemBody::Notice {
                level: Level::Warn,
                code: "UI_HANDLER".into(),
                text: format!("{plugin}: {text}"),
            },
        },
        None => CommandOutcome::Applied { message: None },
    })
}
