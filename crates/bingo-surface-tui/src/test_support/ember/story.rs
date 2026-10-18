//! Prototype turn data folded through the session's event boundary.
use crate::test_support::*;
use bingo_sdk::{CatalogEntry, Reads, ToolLook, ToolTraits, TurnStatus, Upshot};

pub fn scene(name: &str) -> (SessionState, crate::tool_rows::Looks) {
    let data = match name {
        "done" | "ascii-done" | "small-done" | "mode" | "mode-later" => include_str!("done.json"),
        "stop" => include_str!("stop.json"),
        "steer" => include_str!("steer.json"),
        "steer-landed" => include_str!("steer-landed.json"),
        _ => include_str!("live.json"),
    };
    let mut turns: Vec<Value> = serde_json::from_str(data).unwrap();
    for turn in &mut turns {
        clock_origin(turn);
    }
    let mut state = state();
    for (at, turn) in turns.iter().enumerate() {
        fold_turn(&mut state, at, turn);
    }
    (state, crate::tool_rows::Looks::of(&catalogue()))
}

fn clock_origin(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                if matches!(key.as_str(), "t0" | "at" | "end" | "endAt") && value.is_number() {
                    *value = json!(value.as_f64().unwrap() as i64 + super::CLOCK_ORIGIN_MS);
                } else {
                    clock_origin(value);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                clock_origin(value);
            }
        }
        _ => {}
    }
}

fn emit(state: &mut SessionState, at: i64, event: Event) {
    let mut frame = frame(state.seq.0 + 1, event);
    frame.ts = Timestamp::from_millisecond(at).unwrap();
    state.apply(&frame);
}

fn fold_turn(state: &mut SessionState, n: usize, turn: &Value) {
    let id = format!("ember-{n}");
    let t0 = turn["t0"].as_i64().unwrap();
    emit(state, t0, started(&id));
    let mut prompt = user(&format!("{id}-prompt"), turn["prompt"].as_str().unwrap());
    prompt.turn = Some(TurnId::from_raw(&id));
    stamp(&mut prompt, t0, Some(t0));
    emit(state, t0, Event::ItemCompleted { item: prompt });
    for (at, data) in turn["items"].as_array().unwrap().iter().enumerate() {
        let mut item = from_data(&format!("{id}-{at}"), data);
        item.turn = Some(TurnId::from_raw(&id));
        let at = data["at"].as_i64().unwrap();
        let end = data["end"]
            .as_i64()
            .or_else(|| item.status.is_terminal().then_some(at));
        stamp(&mut item, at, end);
        emit(
            state,
            end.unwrap_or(at),
            if item.status.is_terminal() {
                Event::ItemCompleted { item }
            } else {
                Event::ItemStarted { item }
            },
        );
    }
    if turn["reply"].is_object() {
        let reply = &turn["reply"];
        let text = markdown(reply["blocks"].as_array().unwrap());
        let mut item = assistant(&format!("{id}-reply"), &text, ItemStatus::Completed);
        item.turn = Some(TurnId::from_raw(&id));
        stamp(
            &mut item,
            reply["at"].as_i64().unwrap(),
            Some(turn["endAt"].as_f64().unwrap() as i64),
        );
        emit(
            state,
            item.completed_at.unwrap().as_millisecond(),
            Event::ItemCompleted { item },
        );
    }
    let status = match turn["state"].as_str().unwrap() {
        "done" => Some(TurnStatus::Completed),
        "stopped" => Some(TurnStatus::Interrupted {
            reason: bingo_sdk::InterruptReason::UserCancel,
        }),
        _ => None,
    };
    if let Some(status) = status {
        emit(
            state,
            turn["endAt"].as_f64().unwrap() as i64,
            completed(&id, status),
        );
    }
}

fn stamp(item: &mut Item, at: i64, end: Option<i64>) {
    item.started_at = Timestamp::from_millisecond(at).unwrap();
    item.completed_at = end.map(|ms| Timestamp::from_millisecond(ms).unwrap());
}

fn from_data(id: &str, data: &Value) -> Item {
    match data["k"].as_str().unwrap() {
        "narr" => assistant(id, data["text"].as_str().unwrap(), ItemStatus::Completed),
        "think" => item(
            id,
            ItemStatus::Completed,
            ItemBody::Reasoning {
                text: data["text"].as_str().unwrap().into(),
                provider_metadata: Default::default(),
            },
        ),
        "you" => user(id, data["text"].as_str().unwrap()),
        "step" => call(id, data),
        kind => panic!("unported prototype item {kind}"),
    }
}

fn call(id: &str, data: &Value) -> Item {
    let status = match data["st"].as_str().unwrap() {
        "run" => ItemStatus::Running,
        "fail" => ItemStatus::Failed,
        "cut" | "skip" => ItemStatus::Interrupted,
        _ => ItemStatus::Completed,
    };
    let upshot = if data["add"].is_number() || data["del"].is_number() {
        Upshot::Diff {
            added: data["add"].as_u64().unwrap_or(0) as u32,
            removed: data["del"].as_u64().unwrap_or(0) as u32,
        }
    } else {
        Upshot::plain(data["res"].as_str().unwrap_or_default())
    };
    let tail = data["tail"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join("\n");
    let output = status
        .is_terminal()
        .then(|| ToolOutput::text(&tail).with_upshot(upshot));
    let mut item = tool(
        id,
        data["tool"].as_str().unwrap(),
        json!({"target":data["arg"]}),
        output,
        status,
    );
    if let ItemBody::ToolCall { progress, .. } = &mut item.body {
        *progress = (!tail.is_empty()).then_some(tail);
    }
    item
}

pub fn catalogue() -> Vec<CatalogEntry> {
    let data: Value = serde_json::from_str(include_str!("tools.json")).unwrap();
    let entries: Vec<_> = data
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, value)| {
            let mut look = ToolLook::new(
                value["mark"].as_str().unwrap().chars().next().unwrap(),
                value["verb"].as_str().unwrap_or(name),
            );
            look.ascii = value["ascii"].as_str().and_then(|s| s.chars().next());
            let reads = match value["arg"].as_str() {
                Some("path") => Reads::Path,
                Some("cmd") => Reads::Command,
                _ => Reads::Text,
            };
            look = look.target("target", reads);
            let traits = if value["ro"] == true {
                ToolTraits::read_only()
            } else {
                ToolTraits::default()
            };
            CatalogEntry {
                id: name.clone(),
                label: name.clone(),
                meta: json!({"look":look,"traits":traits}),
            }
        })
        .collect();
    entries
}

fn markdown(blocks: &[Value]) -> String {
    blocks
        .iter()
        .map(|block| {
            if let Some(text) = block["p"].as_str() {
                return text.to_string();
            }
            if let Some(text) = block["code"].as_str() {
                return format!(
                    "```{}\n{text}\n```",
                    block["lang"].as_str().unwrap_or_default()
                );
            }
            if let Some(items) = block["ul"].as_array() {
                return items
                    .iter()
                    .map(|v| format!("- {}", v.as_str().unwrap()))
                    .collect::<Vec<_>>()
                    .join("\n");
            }
            panic!("unported answer block {block}")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
