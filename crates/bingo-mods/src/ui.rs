//! The UI noun publishes places and reads them through the host client.
use bingo_plugin_host::{
    host::{Operations, Place},
    ops::{self, Caller},
};
use bingo_sdk::{
    render::{Component, RenderRequest, SurfaceName, Viewport},
    slot::{Shown, Slot},
    *,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

pub(crate) struct Ui {
    runtime: Arc<crate::dollar::Runtime>,
}
pub(crate) fn install(
    host: &HostHandle,
    runtime: Arc<crate::dollar::Runtime>,
) -> Result<(), KernelError> {
    host.open_service("bingo.plugin.ui", Arc::new(Ui { runtime }))
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct Open {
    session: SessionId,
    id: String,
    title: String,
    #[serde(default = "yes")]
    focus: bool,
    #[serde(default = "yes")]
    close_on_escape: bool,
    #[serde(default)]
    hold_toasts: bool,
    #[serde(default = "rows")]
    rows: u16,
    #[serde(default)]
    props: Value,
}
fn yes() -> bool {
    true
}
fn rows() -> u16 {
    8
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
struct Close {
    session: SessionId,
    id: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
struct Toast {
    session: SessionId,
    text: String,
    #[serde(default = "duration")]
    duration: u64,
}
fn duration() -> u64 {
    4000
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
struct Resolve {
    surface: SurfaceName,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
struct Notice {
    text: String,
    #[serde(default = "info")]
    level: Level,
}
fn info() -> Level {
    Level::Info
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
struct Ask {
    session: SessionId,
    kind: InteractionKind,
    answers: Vec<AnswerSpec>,
}

#[async_trait::async_trait]
impl WireService for Ui {
    fn face(&self) -> Option<Face> {
        let mut face = Face::new("ui")
            .method::<Place, ()>("status", false)
            .method::<Toast, ()>("toast", false)
            .method::<Toast, ()>("notify", false)
            .method::<Open, Value>("open", false)
            .method::<Close, ()>("close", false)
            .method::<catalogue::Session, Vec<Slot>>("panes", false)
            .method::<render::Invalidate, ()>("invalidate", false)
            .method::<Resolve, Value>("resolve", false)
            .method::<Notice, ()>("notice", false)
            .method::<Ask, Answer>("ask", false);
        for method in &mut face.methods {
            method.entry.pinned = &["/plugin"];
        }
        Some(face)
    }
    async fn call(&self, method: &str, params: Value) -> Result<Value, ServiceError> {
        let caller = ops::caller().map_err(ServiceError)?;
        match method {
            "status" => place(&caller, parse(params)?).await,
            "open" => open(&caller, parse(params)?).await,
            "close" => close(&caller, parse(params)?).await,
            "toast" | "notify" => toast(&caller, parse(params)?, method == "notify").await,
            "invalidate" => {
                let input: render::Invalidate = parse(params)?;
                caller
                    .host
                    .invalidate(&input.session, input.requests)
                    .await
                    .map_err(failed)?;
                Ok(Value::Null)
            }
            "resolve" => Ok(json!(view::table::elements(
                parse::<Resolve>(params)?.surface
            ))),
            "panes" => {
                let input: catalogue::Session = parse(params)?;
                let state = self
                    .runtime
                    .snapshot(&caller.host, &input.session)
                    .await
                    .map_err(ServiceError)?;
                Ok(json!(
                    slot::placed(&state)
                        .into_iter()
                        .filter(|placed| placed.plugin == caller.plugin)
                        .filter_map(|placed| matches!(placed.slot, Slot::Pane { .. })
                            .then_some(placed.slot))
                        .collect::<Vec<_>>()
                ))
            }
            "notice" => {
                let input: Notice = parse(params)?;
                caller
                    .host
                    .notice(input.level, "UI_NOTICE", &input.text)
                    .await
                    .map_err(failed)?;
                Ok(Value::Null)
            }
            "ask" => {
                let input: Ask = parse(params)?;
                Ok(json!(
                    caller
                        .host
                        .ask(&input.session, input.kind, input.answers)
                        .await
                        .map_err(failed)?
                ))
            }
            _ => Err(ServiceError(format!("unknown ui method: {method}"))),
        }
    }
}
async fn open(caller: &Caller, input: Open) -> Result<Value, ServiceError> {
    let request = RenderRequest {
        component: Component::Pane,
        request_id: input.id.clone(),
        surface: SurfaceName::Terminal,
        props: input.props,
        viewport: Viewport {
            columns: 0,
            rows: input.rows,
        },
    };
    let slot = Slot::Pane {
        id: input.id.clone(),
        title: input.title,
        focus: input.focus,
        close_on_escape: input.close_on_escape,
        hold_toasts: input.hold_toasts,
        asked: render::asked(),
        rows: input.rows,
        view: Shown::Pull { request },
    };
    place(
        caller,
        Place {
            session: Some(input.session),
            slot: input.id,
            view: Some(slot),
        },
    )
    .await?;
    Ok(json!({}))
}
async fn place(caller: &Caller, input: Place) -> Result<Value, ServiceError> {
    Operations {
        plugin: &caller.plugin,
        host: &caller.host,
        state: &caller.state,
    }
    .answer("host/place", json!(input))
    .await
    .map_err(|error| ServiceError(error.message))
}

async fn close(caller: &Caller, input: Close) -> Result<Value, ServiceError> {
    place(
        caller,
        Place {
            session: Some(input.session.clone()),
            slot: input.id.clone(),
            view: None,
        },
    )
    .await?;
    caller.state.forget(&input.session, &input.id);
    Ok(Value::Null)
}
async fn toast(caller: &Caller, input: Toast, alert: bool) -> Result<Value, ServiceError> {
    let until = jiff::Timestamp::now()
        .checked_add(std::time::Duration::from_millis(input.duration))
        .map_err(failed)?;
    place(
        caller,
        Place {
            session: Some(input.session),
            slot: format!("toast:{}", ItemId::mint()),
            view: Some(Slot::Toast {
                text: input.text,
                until,
                alert,
            }),
        },
    )
    .await
}
fn parse<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, ServiceError> {
    serde_json::from_value(value).map_err(failed)
}
fn failed(error: impl std::fmt::Display) -> ServiceError {
    ServiceError(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn opening_publishes_a_pulled_pane_and_cannot_spoof_asked() {
        let root = tempfile::tempdir().unwrap();
        let host = testing::TestHost::default();
        host.on_signal(|_, _, _, _| async { Ok(()) });
        let handle = host.handle();
        crate::dollar::Runtime::install(
            &handle,
            root.path().into(),
            std::time::Duration::from_secs(1),
        )
        .unwrap();
        let ui = handle.service_wire("bingo.plugin.ui").unwrap();
        let store = bingo_plugin_host::state::Store::new(root.path(), "counter");
        for asked in [false, true] {
            let result = render::under_input(
                asked,
                ops::scope(
                    &handle,
                    "counter",
                    &store,
                    ui.call(
                        "open",
                        json!({"session":"ses_ui","id":"counter","title":"Counter", "asked":true}),
                    ),
                ),
            )
            .await
            .unwrap();
            assert_eq!(result, json!({}));
            let calls = host.calls();
            let payload = calls
                .iter()
                .rev()
                .find_map(|call| match call {
                    testing::HostCall::Signal { payload, .. } => Some(payload),
                    _ => None,
                })
                .unwrap();
            let slot: Slot = serde_json::from_value(payload["slots"][0].clone()).unwrap();
            assert!(
                matches!(slot, Slot::Pane {asked:was_asked, view:Shown::Pull{request}, ..} if was_asked == asked && request.request_id == "counter" && request.component == Component::Pane)
            );
        }
    }
}
