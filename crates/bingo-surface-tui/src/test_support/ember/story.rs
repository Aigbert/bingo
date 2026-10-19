//! Prototype turn data folded through the session's event boundary.
use crate::test_support::*;
use bingo_sdk::{CatalogEntry, Reads, ToolLook, ToolTraits, TurnStatus, Upshot};

pub fn scene(name: &str) -> (SessionState, crate::tool_rows::Looks) {
    let turns = turns(name);
    let mut state = state();
    for (at, turn) in turns.iter().enumerate() {
        fold_turn(&mut state, at, turn);
    }
    (state, crate::tool_rows::Looks::of(&catalogue()))
}

pub fn turns(name: &str) -> Vec<Value> {
    let data = match name {
        "done" | "done-detail" | "done-well" | "done-folded" | "ledger" | "ledger-up" | "find"
        | "find-up" | "find-back" | "ascii-done" | "small-done" | "mode" | "mode-later" => {
            include_str!("done.json")
        }
        "stop" => include_str!("stop.json"),
        "steer" => include_str!("steer.json"),
        "steer-landed" => include_str!("steer-landed.json"),
        _ => include_str!("live.json"),
    };
    let mut turns: Vec<Value> = serde_json::from_str(data).unwrap();
    for turn in &mut turns {
        clock_origin(turn);
    }
    turns
}

/// The prototype's small scripts use offsets from the turn's start.
pub fn script(state: &mut SessionState, name: &str, title: &str, start: Timestamp, elapsed: i64) {
    let id = format!("script-{name}");
    emit(state, start.as_millisecond(), started(&id));
    let mut prompt = user("script-prompt", title);
    prompt.turn = Some(TurnId::from_raw(&id));
    stamp(
        &mut prompt,
        start.as_millisecond(),
        Some(start.as_millisecond()),
    );
    emit(
        state,
        start.as_millisecond(),
        Event::ItemCompleted { item: prompt },
    );
    let now = start.as_millisecond() + elapsed;
    for (at, end, mut item) in script_steps(name, title, start) {
        if at > now {
            continue;
        }
        if end > now {
            item.status = ItemStatus::Running;
            stamp(&mut item, at, None);
        }
        if item.status.is_terminal() {
            emit(state, end, Event::ItemCompleted { item });
        } else {
            emit(state, at, Event::ItemStarted { item });
        }
    }
    if name == "pastScript" {
        emit(
            state,
            start.as_millisecond() + 1300,
            completed(&id, TurnStatus::Completed),
        );
    }
}

/// A script's items as the provider streams them: each one starts at its
/// moment and completes at its end.
pub fn script_timeline(name: &str, title: &str, start: Timestamp) -> Vec<(i64, Event)> {
    let mut timeline = vec![];
    for (at, end, item) in script_steps(name, title, start) {
        let mut running = item.clone();
        running.status = ItemStatus::Running;
        stamp(&mut running, at, None);
        timeline.push((at, Event::ItemStarted { item: running }));
        if item.status.is_terminal() {
            timeline.push((end, Event::ItemCompleted { item }));
        }
    }
    timeline
}

/// Every item of a script, finished, with the moments it starts and ends.
fn script_steps(name: &str, title: &str, start: Timestamp) -> Vec<(i64, i64, Item)> {
    let id = format!("script-{name}");
    let shown = if title.chars().count() > 24 {
        format!("{}…", title.chars().take(24).collect::<String>())
    } else {
        title.into()
    };
    let mut steps = vec![];
    for (n, mut data) in super::world::rows(name).into_iter().enumerate() {
        let at = start.as_millisecond() + data["at"].as_i64().unwrap();
        data["at"] = json!(at);
        if let Some(end) = data["end"].as_i64() {
            data["end"] = json!(start.as_millisecond() + end);
        }
        if let Some(text) = data["text"].as_str() {
            data["text"] = json!(
                text.replace("{title}", title)
                    .replace("{text}", &shown)
                    .replace("{T2P}", "修好它，加一个回归测试")
            );
        }
        let mut item = from_data(&format!("{id}-{n}"), &data);
        item.turn = Some(TurnId::from_raw(&id));
        let end = data["end"]
            .as_i64()
            .unwrap_or(at + data["ms"].as_i64().unwrap_or(0));
        let ended = item.status.is_terminal().then_some(end);
        stamp(&mut item, at, ended);
        steps.push((at, end, item));
    }
    steps
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

pub fn fold_turn(state: &mut SessionState, n: usize, turn: &Value) {
    let id = format!("ember-{n}");
    let t0 = turn["t0"].as_i64().unwrap();
    emit(state, t0, started(&id));
    let mut prompt = user(&format!("{id}-prompt"), turn["prompt"].as_str().unwrap());
    prompt.turn = Some(TurnId::from_raw(&id));
    if let ItemBody::User { parts, .. } = &mut prompt.body {
        for _ in turn["chips"].as_array().into_iter().flatten() {
            parts.push(ContentPart::Image(
                bingo_sdk::Image::from_bytes("image/png", b"fixture").unwrap(),
            ));
        }
    }
    stamp(&mut prompt, t0, Some(t0));
    emit(state, t0, Event::ItemCompleted { item: prompt });
    for (at, data) in turn["items"].as_array().unwrap().iter().enumerate() {
        let mut item = from_data(&format!("{id}-{at}"), data);
        item.turn = Some(TurnId::from_raw(&id));
        let at = data["at"].as_i64().unwrap();
        let end = data["end"]
            .as_i64()
            .or_else(|| data["ms"].as_i64().map(|ms| at + ms))
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
    fold_reply(state, &id, turn);
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

fn fold_reply(state: &mut SessionState, id: &str, turn: &Value) {
    if turn["reply"].is_object() {
        let reply = &turn["reply"];
        let text = markdown(reply["blocks"].as_array().unwrap());
        let text: String = text
            .chars()
            .take(
                reply["revealed"]
                    .as_u64()
                    .map_or(usize::MAX, |n| n as usize),
            )
            .collect();
        let finished = turn["state"] == "done";
        let mut item = assistant(
            &format!("{id}-reply"),
            &text,
            if finished {
                ItemStatus::Completed
            } else {
                ItemStatus::Running
            },
        );
        item.turn = Some(TurnId::from_raw(id));
        stamp(
            &mut item,
            reply["at"].as_i64().unwrap(),
            finished.then(|| turn["endAt"].as_f64().unwrap() as i64),
        );
        emit(
            state,
            item.completed_at
                .unwrap_or(item.started_at)
                .as_millisecond(),
            if finished {
                Event::ItemCompleted { item }
            } else {
                Event::ItemStarted { item }
            },
        );
    }
}

pub(super) fn stamp(item: &mut Item, at: i64, end: Option<i64>) {
    item.started_at = Timestamp::from_millisecond(at).unwrap();
    if let ItemBody::ToolCall { began_at, .. } = &mut item.body {
        *began_at = Some(item.started_at);
    }
    item.completed_at = end.map(|ms| Timestamp::from_millisecond(ms).unwrap());
}

pub(super) fn from_data(id: &str, data: &Value) -> Item {
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
    let output = status.is_terminal().then(|| {
        let text = data["out"]
            .as_array()
            .map(|lines| {
                lines
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_else(|| tail.clone());
        let mut output = ToolOutput::text(text).with_upshot(upshot);
        output.display = display(data);
        output
    });
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

/// Provider-owned items only; keys supply prompts, steers and interruptions.
pub fn provider_events(n: usize, turn: &Value) -> Vec<(i64, Event)> {
    let id = format!("ember-{n}");
    let mut events = Vec::new();
    for (index, data) in turn["items"].as_array().unwrap().iter().enumerate() {
        if data["k"] == "you" {
            continue;
        }
        let mut item = from_data(&format!("{id}-{index}"), data);
        item.turn = Some(TurnId::from_raw(&id));
        let at = data["at"].as_i64().unwrap();
        let end = data["end"]
            .as_i64()
            .or_else(|| item.status.is_terminal().then_some(at));
        stamp(&mut item, at, end);
        if item.status == ItemStatus::Interrupted {
            item.status = ItemStatus::Running;
            item.completed_at = None;
            if let ItemBody::ToolCall { output, .. } = &mut item.body {
                *output = None;
            }
            events.push((at, Event::ItemStarted { item }));
        } else if data["k"] == "step" && item.is_terminal() {
            let mut running = item.clone();
            running.status = ItemStatus::Running;
            running.completed_at = None;
            if let ItemBody::ToolCall { output, .. } = &mut running.body {
                *output = None;
            }
            events.push((at, Event::ItemStarted { item: running }));
            events.push((end.unwrap(), Event::ItemCompleted { item }));
        } else {
            let event = if item.is_terminal() {
                Event::ItemCompleted { item }
            } else {
                Event::ItemStarted { item }
            };
            events.push((at, event));
        }
    }
    let mut reply = state();
    fold_reply(&mut reply, &id, turn);
    for item in reply.transcript.items() {
        let event = if item.is_terminal() {
            Event::ItemCompleted {
                item: item.as_ref().clone(),
            }
        } else {
            Event::ItemStarted {
                item: item.as_ref().clone(),
            }
        };
        events.push((
            item.completed_at
                .unwrap_or(item.started_at)
                .as_millisecond(),
            event,
        ));
    }
    if turn["state"] == "done" {
        events.push((
            turn["endAt"].as_f64().unwrap() as i64,
            completed(&id, TurnStatus::Completed),
        ));
    }
    events.sort_by_key(|(at, _)| *at);
    events
}

fn display(data: &Value) -> Option<bingo_sdk::View> {
    if let Some(diff) = data["diff"].as_array() {
        return diff_display(diff);
    }
    let code = data["code"].as_object()?;
    let lines = code["lines"].as_array()?;
    let start = code["start"].as_u64()? as u32;
    let source = lines
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join("\n");
    let of = code["of"].as_u64()? as usize;
    let mut file: Vec<String> = vec![String::new(); start as usize - 1];
    file.extend(source.split('\n').map(str::to_string));
    file.resize(of, "unread source line".into());
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("source.rs"), file.join("\n")).unwrap();
    tool_output(
        &bingo_tool_fs::ReadTool,
        json!({"file_path":"source.rs", "offset":start, "limit":lines.len()}),
        dir.path(),
    )
    .display
}

fn diff_display(rows: &[Value]) -> Option<bingo_sdk::View> {
    let first = rows.first()?["n"].as_u64()? as usize;
    let prefix = "unchanged\n".repeat(first.saturating_sub(1));
    let side = |excluded: &str| {
        prefix.clone()
            + &rows
                .iter()
                .filter(|row| row["s"] != excluded)
                .map(|row| format!("{}\n", row["t"].as_str().unwrap()))
                .collect::<String>()
    };
    let old = side("+");
    let new = side("-");
    let dir = tempfile::tempdir().unwrap();
    let output = if old.is_empty() {
        tool_output(
            &bingo_tool_fs::WriteTool,
            json!({"file_path":"source.rs","content":new}),
            dir.path(),
        )
    } else {
        std::fs::write(dir.path().join("source.rs"), &old).unwrap();
        tool_output(
            &bingo_tool_fs::EditTool,
            json!({"file_path":"source.rs","old_string":old,"new_string":new}),
            dir.path(),
        )
    };
    output.display
}

fn tool_output(tool: &dyn bingo_sdk::Tool, input: Value, cwd: &std::path::Path) -> ToolOutput {
    use std::sync::Arc;
    let cx = bingo_sdk::ToolContext {
        call_id: "fixture".into(),
        session: SessionId::from_raw("fixture"),
        turn: TurnId::from_raw("fixture"),
        item: ItemId::from_raw("fixture"),
        cwd: cwd.to_path_buf(),
        cancel: bingo_sdk::CancellationToken::new(),
        env: Arc::new(bingo_sdk::Env {
            home: cwd.into(),
            config_dir: cwd.into(),
            data_dir: cwd.into(),
        }),
        host: bingo_sdk::testing::NoHost::handle(),
        call: Arc::new(FixtureCall),
    };
    // Captures also run inside async tests; the tool owns an independent runtime.
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(tool.call(input, &cx))
                    .unwrap()
            })
            .join()
            .unwrap()
    })
}

#[derive(Debug)]
struct FixtureCall;
#[async_trait::async_trait]
impl bingo_sdk::Prompter for FixtureCall {
    async fn ask(
        &self,
        _: bingo_sdk::InteractionKind,
        _: Vec<bingo_sdk::AnswerSpec>,
    ) -> Result<bingo_sdk::Answer, bingo_sdk::KernelError> {
        panic!("filesystem fixture never asks")
    }
}
#[async_trait::async_trait]
impl bingo_sdk::ToolHost for FixtureCall {
    fn progress(&self, _: &ItemId, _: String) {}
    async fn record(&self, _: ItemBody) -> Result<ItemId, bingo_sdk::KernelError> {
        panic!("filesystem fixture never records")
    }
}
