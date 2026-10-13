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
struct Caller {
    plugin: String,
    host: HostHandle,
    state: Store,
}
struct Ops(&'static str);

pub fn install(host: &HostHandle) -> Result<(), KernelError> {
    for noun in ["state", "store", "ui", "model", "session"] {
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
    let (noun, verb) =
        route(method, &params).ok_or_else(|| format!("the host does not serve {method}"))?;
    let grant = granted(host, plugin).await?;
    admits(&grant, noun, verb, false).map_err(|error| error.to_string())?;
    let wire = host
        .service_wire(&key(noun))
        .ok_or_else(|| format!("{noun}: host face is unavailable"))?;
    let caller = Caller {
        plugin: plugin.into(),
        host: host.clone(),
        state: state.clone(),
    };
    CALLER
        .scope(caller, wire.call(verb, params))
        .await
        .map_err(|error| error.to_string())
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
    let face = wire.face();
    let noun = face.as_ref().map(|face| face.noun.as_str()).unwrap_or(key);
    let unsafe_call = face
        .as_ref()
        .and_then(|face| face.find(method))
        .is_none_or(|method| method.unsafe_call);
    admits(&granted(host, plugin).await?, noun, method, unsafe_call)
        .map_err(|error| error.to_string())?;
    let result = wire
        .call(method, params.get("params").cloned().unwrap_or(Value::Null))
        .await
        .map_err(|error| error.to_string())?;
    Ok(json!({"result":result}))
}
fn route(method: &str, params: &Value) -> Option<(&'static str, &'static str)> {
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
        _ => None,
    }
}
fn wire_method(noun: &str, verb: &str) -> Option<&'static str> {
    match (noun, verb) {
        ("state" | "store", "get") => Some("host/state/get"),
        ("state" | "store", "set") => Some("host/state/set"),
        ("ui", "status") => Some("host/place"),
        ("model", "complete") => Some("host/model/complete"),
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
            "model" => face.method::<Complete, Completed>("complete", true),
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
