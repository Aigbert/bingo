//! Noun faces adapt calls onto host doors and the client's fold.
use super::{
    client::Sessions,
    params::*,
    table::{Door, ROWS},
};
use bingo_plugin_host::{
    host::Operations,
    ops::{self, Caller},
};
use bingo_sdk::*;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};

pub struct Runtime {
    timers: super::clock::Timers,
    every_floor: std::time::Duration,
    pub(super) registered: super::register::Registrations,
    sessions: Sessions,
    root: PathBuf,
}
struct Noun {
    noun: &'static str,
    runtime: Arc<Runtime>,
}
impl Runtime {
    pub fn install(
        host: &HostHandle,
        root: PathBuf,
        every_floor: std::time::Duration,
    ) -> Result<Arc<Self>, KernelError> {
        let runtime = Arc::new(Self {
            timers: Default::default(),
            every_floor,
            registered: Default::default(),
            sessions: Sessions::default(),
            root,
        });
        let nouns: std::collections::BTreeSet<_> = ROWS.iter().map(|row| row.noun).collect();
        for noun in nouns {
            host.open_service(
                &format!("bingo.plugin.{noun}"),
                Arc::new(Noun {
                    noun,
                    runtime: runtime.clone(),
                }),
            )?;
        }
        crate::ui::install(host, runtime.clone())?;
        Ok(runtime)
    }
    pub(crate) fn tools(&self) -> Vec<Arc<dyn Tool>> {
        self.registered.tools()
    }
    pub(crate) fn commands(&self) -> Vec<Arc<dyn Command>> {
        self.registered.commands()
    }
    pub async fn stop(&self) {
        self.timers.stop().await;
        self.sessions.stop().await;
    }
    pub async fn end(&self, session: &SessionId) {
        self.timers.end(session).await;
        self.sessions.remove(session).await;
    }

    pub async fn surface(
        &self,
        host: &HostHandle,
        input: catalogue::Attachment,
        attached: bool,
    ) -> Result<(), String> {
        if input.client.surface == "mods" {
            return Ok(());
        }
        let live = self.sessions.get(host, &input.session).await?;
        let key = format!(
            "surface/{}",
            serde_json::to_string(&input.client).map_err(failed)?
        );
        let snapshot = live.snapshot();
        let count = snapshot
            .signals
            .get("bingo.mods")
            .and_then(|values| values.get(&key))
            .and_then(|value| value["count"].as_u64())
            .unwrap_or(0);
        let count = if attached {
            count.saturating_add(1)
        } else {
            count.saturating_sub(1)
        };
        let value = if count == 0 {
            Value::Null
        } else {
            json!({"client":input.client,"count":count})
        };
        host.signal(&input.session, "bingo.mods", &key, value.clone())
            .await
            .map_err(failed)?;
        live.signal(&key, &value).await
    }

    async fn call(&self, caller: &Caller, method: &str, params: Value) -> Result<Value, String> {
        if (method.starts_with("state.") || method.starts_with("store.")) && method != "store.keys"
        {
            let global = params.get("scope").and_then(Value::as_str) == Some("global");
            if global != method.starts_with("store.") {
                return Err("operation scope does not match its noun".into());
            }
        }
        match method {
            "store.keys" | "store.delete" => store(caller, method, params).await,
            "clock.now" => Ok(json!(jiff::Timestamp::now().to_string())),
            "clock.sleep" => {
                tokio::time::sleep(std::time::Duration::from_millis(
                    parse::<Millis>(params)?.ms,
                ))
                .await;
                Ok(Value::Null)
            }
            "clock.after" | "clock.every" => {
                self.timer(caller, parse(params)?, method == "clock.every")
                    .await
            }
            "session.version" => Ok(json!(env!("CARGO_PKG_VERSION"))),
            "session.root" => Ok(json!(self.root)),
            "session.messages" => self.messages(caller, parse(params)?).await,
            "session.id" | "session.cwd" | "session.model" | "session.turns" | "session.usage"
            | "session.surfaces" | "config.list" | "config.read" | "prompt.read" => {
                self.read(caller, method, params).await
            }
            "session.append" | "session.extend" => operation(caller, "host/extend", params).await,
            "session.deliver" => operation(caller, "host/deliver", params).await,
            "model.classify" => super::model::classify(caller, parse(params)?).await,
            "model.fork" => self.fork(caller, parse(params)?).await,
            "model.complete" => operation(caller, "host/model/complete", params).await,
            "state.get" | "store.get" => operation(caller, "host/state/get", params).await,
            "state.set" | "store.set" => operation(caller, "host/state/set", params).await,
            "session.send" => send(caller, parse(params)?).await,
            "session.authorize" => authorize(caller, parse(params)?).await,
            "tool.register" => Ok(json!(self.registered.tool(caller, parse(params)?)?)),
            "command.register" => Ok(json!(self.registered.command(caller, parse(params)?)?)),
            "tool.list" => catalog(caller, CatalogKind::Tools).await,
            "command.list" => catalog(caller, CatalogKind::Commands).await,
            "tool.call" => invoke(caller, parse(params)?).await,
            "config.set" => setting(caller, parse(params)?).await,
            "prompt.submit" | "command.run" | "session.compact" | "turn.abort" => {
                self.input(caller, method, params).await
            }
            _ => Err(format!("{method}: runtime operation is unavailable")),
        }
    }
    async fn messages(&self, caller: &Caller, input: Messages) -> Result<Value, String> {
        let live = self.sessions.get(&caller.host, &input.session).await?;
        Ok(json!(
            live.handle.history(input.page).await.map_err(failed)?
        ))
    }
    async fn read(&self, caller: &Caller, method: &str, params: Value) -> Result<Value, String> {
        let input: catalogue::Session = parse(params.clone())?;
        let live = self.sessions.get(&caller.host, &input.session).await?;
        super::read::read(
            &live.snapshot(),
            method,
            params.get("key").and_then(Value::as_str),
        )
    }
    async fn fork(&self, caller: &Caller, input: Fork) -> Result<Value, String> {
        let live = self.sessions.get(&caller.host, &input.session).await?;
        let history = live
            .handle
            .history(HistoryPage::default())
            .await
            .map_err(failed)?;
        let request = super::model::fork_request(&history.items, &input.prompt);
        let events =
            super::model::complete(caller, &input.session, input.turn.as_ref(), request).await?;
        Ok(json!(bingo_plugin_host::host::Completed { events }))
    }

    async fn timer(&self, caller: &Caller, input: Timer, every: bool) -> Result<Value, String> {
        let registry = caller
            .host
            .service(&bingo_plugin_host::callback::KEY)
            .ok_or("callback registry is unavailable")?;
        let executor = registry.get(&caller.plugin)?;
        let state = self
            .sessions
            .get(&caller.host, &input.session)
            .await?
            .snapshot();
        let cx = HookContext {
            session: input.session,
            turn: None,
            item: None,
            cwd: state.summary.cwd.into(),
            provider: None,
            model: state.summary.model,
            host: caller.host.clone(),
        };
        let requested = std::time::Duration::from_millis(input.ms);
        let period = if every {
            requested.max(self.every_floor)
        } else {
            requested
        };
        Ok(json!(self.timers.start(
            cx,
            executor,
            input.callback,
            period,
            every
        )?))
    }

    async fn input(&self, caller: &Caller, method: &str, params: Value) -> Result<Value, String> {
        let session: catalogue::Session = parse(params.clone())?;
        let live = self.sessions.get(&caller.host, &session.session).await?;
        let intent = IntentId::mint();
        if method == "turn.abort" {
            live.handle.interrupt(intent, InterruptScope::Head);
            return Ok(Value::Null);
        }
        let input = match method {
            "prompt.submit" => {
                let mut input = parse::<Submit>(params)?.input;
                bind_origin(caller, &mut input);
                input
            }
            "command.run" => {
                let input: Run = parse(params)?;
                Input::Action {
                    action: Action {
                        name: input.name,
                        args: input.args,
                    },
                }
            }
            _ => Input::Action {
                action: Action {
                    name: "compact".into(),
                    args: Value::Null,
                },
            },
        };
        live.handle.submit(intent.clone(), input);
        Ok(json!(intent))
    }
}
#[async_trait::async_trait]
impl WireService for Noun {
    fn face(&self) -> Option<Face> {
        let mut face = Face::new(self.noun);
        for row in ROWS
            .iter()
            .filter(|row| row.noun == self.noun && matches!(row.door, Door::Face))
        {
            let mut entry = catalogue::Entry::face(
                format!("{}.{}", row.noun, row.method),
                schema(row.input),
                schema(row.result),
            );
            entry.pinned = &["/plugin"];
            face.methods.push(service::Method {
                entry,
                unsafe_call: row.unsafe_call,
            });
        }
        Some(face)
    }
    async fn call(&self, method: &str, params: Value) -> Result<Value, ServiceError> {
        let caller = ops::caller().map_err(ServiceError)?;
        self.runtime
            .call(&caller, &format!("{}.{method}", self.noun), params)
            .await
            .map_err(ServiceError)
    }
}
fn schema(shape: fn(&mut schemars::SchemaGenerator) -> schemars::Schema) -> schemars::Schema {
    let mut generator = schemars::SchemaGenerator::default();
    let mut schema = shape(&mut generator);
    schema.insert(
        "$defs".into(),
        Value::Object(generator.take_definitions(true)),
    );
    schema
}
fn parse<T: DeserializeOwned>(input: Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(failed)
}
fn failed(error: impl std::fmt::Display) -> String {
    error.to_string()
}
fn bind_origin(caller: &Caller, input: &mut Input) {
    if let Input::Text { origin, .. } = input {
        *origin = Origin::surface(format!("plugin:{}", caller.plugin));
    }
}
async fn operation(caller: &Caller, method: &str, input: Value) -> Result<Value, String> {
    Operations {
        plugin: &caller.plugin,
        host: &caller.host,
        state: &caller.state,
    }
    .answer(method, input)
    .await
    .map_err(bingo_plugin_host::host::error_message)
}
async fn send(caller: &Caller, mut input: catalogue::Send) -> Result<Value, String> {
    bind_origin(caller, &mut input.input);
    caller
        .host
        .deliver(&input.to, IntentId::mint(), input.input, input.delivery)
        .await
        .map_err(failed)?;
    Ok(Value::Null)
}
async fn authorize(caller: &Caller, input: Authorize) -> Result<Value, String> {
    Ok(json!(
        caller
            .host
            .ask(&input.session, input.kind, input.answers)
            .await
            .map_err(failed)?
    ))
}
async fn invoke(caller: &Caller, input: Invoke) -> Result<Value, String> {
    Ok(json!(
        caller
            .host
            .invoke(&input.session, input.call)
            .await
            .map_err(failed)?
            .output
    ))
}
async fn setting(caller: &Caller, input: Setting) -> Result<Value, String> {
    let change = match input.key.as_str() {
        "model" => SessionChange::Model {
            provider: None,
            model: parse(input.value)?,
        },
        "thinking" => SessionChange::Thinking(parse(input.value)?),
        "title" => SessionChange::Title(parse(input.value)?),
        "key" => SessionChange::Key(parse(input.value)?),
        key => {
            return Err(format!(
                "config.set: {key} has no live reconfiguration door"
            ));
        }
    };
    caller
        .host
        .reconfigure(&input.session, change)
        .await
        .map_err(failed)?;
    Ok(Value::Null)
}

async fn catalog(caller: &Caller, kind: CatalogKind) -> Result<Value, String> {
    Ok(json!(caller.host.catalog(kind).await.map_err(failed)?))
}
async fn store(caller: &Caller, method: &str, params: Value) -> Result<Value, String> {
    if method == "store.keys" {
        return Ok(json!(
            caller
                .state
                .keys("global".into())
                .await
                .map_err(bingo_plugin_host::host::error_message)?
        ));
    }
    let input: bingo_plugin_host::host::StateGet = parse(params)?;
    let removed = caller
        .state
        .delete(input.scope.clone(), input.key.clone())
        .await
        .map_err(bingo_plugin_host::host::error_message)?;
    if removed {
        caller
            .state
            .changed(&caller.host, &input.scope, &input.key)
            .await
            .map_err(bingo_plugin_host::host::error_message)?;
    }
    Ok(json!(removed))
}
