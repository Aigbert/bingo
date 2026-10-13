//! Existing host doors exposed as noun faces, with the caller held out of band.
use crate::{
    grant::{admits, granted},
    host::*,
    state::Store,
};
use async_trait::async_trait;
use bingo_sdk::*;
use serde_json::{Value, json};
use std::sync::Arc;

tokio::task_local! { static CALLER: Caller; }
#[derive(Clone)]
pub struct Caller {
    pub plugin: String,
    pub host: HostHandle,
    pub state: Store,
}
pub fn caller() -> Result<Caller, String> {
    CALLER
        .try_with(Clone::clone)
        .map_err(|_| "plugin operation has no caller".into())
}
pub async fn scope<T>(
    host: &HostHandle,
    plugin: &str,
    state: &Store,
    future: impl std::future::Future<Output = T>,
) -> T {
    CALLER
        .scope(
            Caller {
                plugin: plugin.into(),
                host: host.clone(),
                state: state.clone(),
            },
            future,
        )
        .await
}

struct Ops(&'static str);

pub fn install(host: &HostHandle) -> Result<(), KernelError> {
    for noun in ["state", "store", "ui", "session"] {
        let key = key(noun);
        if host.service_wire(&key).is_none() {
            host.open_service(&key, Arc::new(Ops(noun)))?;
        }
    }
    Ok(())
}
fn key(noun: &str) -> String {
    format!("bingo.plugin.{noun}")
}

pub async fn call(
    host: &HostHandle,
    plugin: &str,
    state: &Store,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    if method == "service/call" {
        let caller = Caller {
            plugin: plugin.into(),
            host: host.clone(),
            state: state.clone(),
        };
        return CALLER.scope(caller, service(host, plugin, params)).await;
    }
    let (noun, verb) = admit(host, plugin, method, &params).await?;
    if method == "host/model/complete" {
        return Operations {
            plugin,
            host,
            state,
        }
        .answer(method, params)
        .await
        .map_err(|error| error.message);
    }
    let wire = host
        .service_wire(&key(noun))
        .ok_or_else(|| format!("{noun}: host face is unavailable"))?;
    let caller = Caller {
        plugin: plugin.into(),
        host: host.clone(),
        state: state.clone(),
    };
    let params = bind(plugin, &key(noun), params)?;
    CALLER
        .scope(caller, wire.call(verb, params))
        .await
        .map_err(|error| error.to_string())
}
/// Both transports consult the same catalogue grant before a host operation.
pub async fn admit<'a>(
    host: &HostHandle,
    plugin: &str,
    method: &'a str,
    params: &Value,
) -> Result<(&'a str, &'a str), String> {
    let (noun, verb) =
        route(method, params).ok_or_else(|| format!("the host does not serve {method}"))?;
    let grant = granted(host, plugin).await?;
    admits(&grant, noun, verb, false).map_err(|error| format!("{plugin}: {error}"))?;
    Ok((noun, verb))
}

async fn service(host: &HostHandle, plugin: &str, params: Value) -> Result<Value, String> {
    let key = params
        .get("key")
        .and_then(Value::as_str)
        .ok_or("service key is required")?;
    let method = params
        .get("method")
        .and_then(Value::as_str)
        .ok_or("service method is required")?;
    let wire = host
        .service_wire(key)
        .ok_or_else(|| format!("service {key} is unavailable"))?;
    crate::grant::admit_service(host, plugin, key, method, wire.as_ref()).await?;
    let input = bind(
        plugin,
        key,
        params.get("params").cloned().unwrap_or(Value::Null),
    )?;
    let result = wire
        .call(method, input)
        .await
        .map_err(|error| error.to_string())?;
    Ok(json!({"result":result}))
}
fn route<'a>(method: &'a str, params: &Value) -> Option<(&'a str, &'a str)> {
    let state = if params.get("scope").and_then(Value::as_str) == Some("global") {
        "store"
    } else {
        "state"
    };
    match method {
        "host/state/get" => Some((state, "get")),
        "host/state/set" => Some((state, "set")),
        "host/place" => Some(("ui", "status")),
        "host/model/complete" => Some(("model", "complete")),
        "host/extend" => Some(("session", "extend")),
        "host/deliver" => Some(("session", "deliver")),
        _ => method.strip_prefix("host/")?.split_once('/'),
    }
}
fn wire_method(noun: &str, verb: &str) -> Option<&'static str> {
    match (noun, verb) {
        ("state" | "store", "get") => Some("host/state/get"),
        ("state" | "store", "set") => Some("host/state/set"),
        ("ui", "status") => Some("host/place"),
        ("session", "extend") => Some("host/extend"),
        ("session", "deliver") => Some("host/deliver"),
        _ => None,
    }
}
#[async_trait]
impl WireService for Ops {
    fn face(&self) -> Option<Face> {
        let face = Face::new(self.0);
        Some(match self.0 {
            "state" | "store" => face
                .method::<StateGet, Option<StateValue>>("get", false)
                .method::<StateSet, StateValue>("set", false),
            "ui" => face.method::<Place, ()>("status", false),
            "session" => face
                .method::<Extend, ()>("extend", false)
                .method::<Deliver, ()>("deliver", false),
            _ => return None,
        })
    }
    async fn call(&self, method: &str, params: Value) -> Result<Value, ServiceError> {
        let caller = CALLER
            .try_with(Clone::clone)
            .map_err(|_| ServiceError("plugin operation has no caller".into()))?;
        let method = wire_method(self.0, method)
            .ok_or_else(|| ServiceError("unknown plugin operation".into()))?;
        if route(method, &params).is_none_or(|(noun, _)| noun != self.0) {
            return Err(ServiceError(
                "operation scope does not match its noun".into(),
            ));
        }
        Operations {
            plugin: &caller.plugin,
            host: &caller.host,
            state: &caller.state,
        }
        .answer(method, params)
        .await
        .map_err(|error| ServiceError(error.message))
    }
}

/// The adapter binds identity before the chain may rewrite an operation.
pub fn bind(plugin: &str, key: &str, mut input: Value) -> Result<Value, String> {
    if !key.starts_with("bingo.plugin.") {
        return Ok(input);
    }
    if let Some(object) = input.as_object_mut() {
        if object.get("plugin").is_some_and(|owner| owner != plugin) {
            return Err("a plugin may not operate on another plugin's state".into());
        }
        object.insert("plugin".into(), Value::String(plugin.into()));
    }
    Ok(input)
}
