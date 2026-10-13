//! Stub links and observations record every crossing in the kv service.
use crate::answer;
use bingo_plugin_rpc::intercept::LinkSpec;
use bingo_plugin_rpc::wire::{HookObservation, HookObserveParams, HookSpec};
use bingo_sdk::{HookMatcher, HookPoint, LinkPoint};
use serde_json::{Value, json};
use std::collections::BTreeMap;
type Store = BTreeMap<String, String>;
pub fn hooks() -> Vec<HookSpec> {
    vec![HookSpec {
        id: "watch".into(),
        matcher: HookMatcher {
            points: vec![HookPoint::Session, HookPoint::Turn],
            tool: None,
        },
    }]
}
pub fn links() -> Vec<LinkSpec> {
    vec![
        LinkSpec {
            point: LinkPoint::ToolCall,
            tool: Some("Bash".into()),
            after: true,
        },
        LinkSpec {
            point: LinkPoint::TurnStop,
            tool: None,
            after: false,
        },
    ]
}
pub fn intercept(
    id: bingo_plugin_rpc::codec::Id,
    method: &str,
    mut params: Value,
    store: &mut Store,
) {
    let point = method.strip_prefix("intercept/").unwrap_or(method);
    if let Some(point) = point.strip_suffix("/after") {
        note(point, "after", store);
        let text = params["result"]["parts"][0]["text"]
            .as_str()
            .unwrap_or_default();
        params["result"]["parts"][0]["text"] = json!(format!("{text} (stub)"));
        answer(id, json!({"result":params["result"]}));
    } else {
        note(point, "before", store);
        if point == "turn.complete" {
            return;
        }
        let command = params["input"]["command"].as_str().unwrap_or_default();
        params["input"]["command"] = json!(format!("{command} --safe"));
        answer(id, json!({"next":params["input"]}));
    }
}
pub fn cancelled(store: &mut Store) {
    note("chain", "cancel", store);
}
/// One observation point: written down, and nothing sent back — the host is
/// not waiting.
pub fn observed(params: &Value, store: &mut Store) {
    let Ok(params) = serde_json::from_value::<HookObserveParams>(params.clone()) else {
        return;
    };
    note(&params.id, watched(&params.observation), store);
}

fn watched(observation: &HookObservation) -> &'static str {
    match observation {
        HookObservation::Turn { .. } => "turn",
        HookObservation::Compact { .. } => "compact",
        HookObservation::Session { .. } => "session",
        HookObservation::Event { .. } => "event",
    }
}

/// Every hook crossing, in the order it arrived, under the `hooks` key of the
/// map `kv` serves. A key that is still absent is a crossing that never
/// happened.
fn note(hook: &str, point: &str, store: &mut Store) {
    let seen = store.entry("hooks".to_string()).or_default();
    if !seen.is_empty() {
        seen.push(',');
    }
    seen.push_str(hook);
    seen.push('/');
    seen.push_str(point);
}
