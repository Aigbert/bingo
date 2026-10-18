//! Scene data comes from the prototype's capture clock; session facts enter as frames.
use super::{Capture, story};
use crate::test_support::*;

pub fn scene(capture: &Capture) -> (SessionState, Ui) {
    let fixtures: Value = serde_json::from_str(include_str!("acting.json")).unwrap();
    let data = &fixtures[&capture.entry.file];
    let now = capture.now();
    let mut summary = summary();
    summary.choice.model = Some("opus-5.5".into());
    summary.provider = Some("anthropic".into());
    summary.effort = Some(bingo_sdk::Effort::High);
    let mut state = SessionState::new(summary);
    for (n, turn) in data["log"].as_array().unwrap().iter().enumerate() {
        if turn["shell"] == true {
            shell(&mut state, n, turn);
        } else {
            story::fold_turn(&mut state, n, turn);
        }
    }
    let (mut ui, _) = crate::test_support::scene();
    ui.catalogs
        .values
        .insert("tools".into(), story::catalogue());
    let keyed = data["keyAt"].as_i64().unwrap();
    ui.keyed_at = now
        .instant
        .checked_sub(std::time::Duration::from_millis(
            (now.wall.as_millisecond() - keyed).max(0) as u64,
        ))
        .unwrap_or(now.instant);
    config(&mut state, data);
    queue(&mut state, &mut ui, data, now);
    draft(&mut ui, data);
    if data["toast"].is_object() {
        let toast = &data["toast"];
        let text = toast["segs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["t"].as_str().unwrap())
            .collect::<String>();
        let at = now.instant
            - std::time::Duration::from_millis(
                (now.wall.as_millisecond() - toast["at"].as_i64().unwrap()) as u64,
            );
        ui.notify_for(
            &text,
            std::time::Duration::from_millis(toast["ms"].as_u64().unwrap()),
            at,
        );
    }
    promotions(&mut state);
    (state, ui)
}

fn config(state: &mut SessionState, data: &Value) {
    let mode = if data["mode"] == 1 {
        "acceptEdits"
    } else {
        "default"
    };
    state.apply(&frame(
        state.seq.0 + 1,
        Event::ConfigChanged {
            config: bingo_sdk::ConfigView {
                kernel: json!({"thinking":"high"}),
                plugins: Default::default(),
            },
        },
    ));
    state.apply(&frame(state.seq.0 + 1, permission_view(mode)));
}

fn queue(state: &mut SessionState, ui: &mut Ui, data: &Value, now: Now) {
    let entries: Vec<_> = data["queued"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(n, q)| bingo_sdk::QueueEntry {
            intent: bingo_sdk::IntentId::from_raw(format!("acting-{n}")),
            position: n as u32 + 1,
            preview: q["text"].as_str().unwrap().into(),
            steerable: q["steer"] == true,
            origin: crate::origin(),
        })
        .collect();
    for (entry, q) in entries.iter().zip(data["queued"].as_array().unwrap()) {
        let age = now.wall.as_millisecond() - q["at"].as_i64().unwrap();
        ui.queue_arrivals.observe(
            &[],
            std::slice::from_ref(entry),
            now.instant - std::time::Duration::from_millis(age as u64),
        );
    }
    state.apply(&frame(
        state.seq.0 + 1,
        Event::QueueChanged {
            revision: 1,
            entries,
        },
    ));
}

fn draft(ui: &mut Ui, data: &Value) {
    let mut text = String::new();
    for chip in data["chips"].as_array().unwrap() {
        let n = ui.pictures.hold(
            &text,
            std::path::PathBuf::from(format!("/fixture/{}.png", chip.as_str().unwrap())),
        );
        text.push_str(&format!("{} ", crate::pictures::placeholder(n)));
    }
    text.push_str(data["input"].as_str().unwrap());
    ui.composer.set(&text);
}

fn shell(state: &mut SessionState, n: usize, turn: &Value) {
    let step = &turn["items"][0];
    let output = step["out"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    let mut shell = item(
        &format!("shell-{n}"),
        ItemStatus::Completed,
        ItemBody::Shell {
            command: turn["prompt"]
                .as_str()
                .unwrap()
                .trim_start_matches('!')
                .into(),
            output,
            exit: Some(0),
            cwd: state.summary.cwd.clone().into(),
        },
    );
    shell.turn = None;
    shell.started_at = Timestamp::from_millisecond(turn["t0"].as_i64().unwrap()).unwrap();
    shell.completed_at =
        Some(Timestamp::from_millisecond(turn["endAt"].as_f64().unwrap() as i64).unwrap());
    state.apply(&frame(
        state.seq.0 + 1,
        Event::ItemCompleted { item: shell },
    ));
}

#[test]
fn acting_full_frames_match_the_prototype() {
    for name in [
        "live-steered",
        "live-stopped",
        "stop",
        "stop-resent",
        "steer",
        "steer-landed",
        "shell",
        "shell-ls",
        "image",
        "image-sent",
    ] {
        let spinners: &[(u16, u16)] = match name {
            "live-steered" => &[(25, 3)],
            "steer-landed" => &[(24, 3)],
            _ => &[],
        };
        let capture = Capture::load(name).spinners(spinners);
        capture.render(|| {
            let (state, mut ui) = scene(&capture);
            let size = capture.size();
            let backend = drawn(
                size.width,
                size.height,
                &solo(&state),
                &mut ui,
                capture.now(),
            );
            super::assert_band(&capture, super::Band::Frame, backend.buffer());
        });
    }
}

fn promotions(state: &mut SessionState) {
    let ids: Vec<_> = state
        .transcript
        .items()
        .iter()
        .filter(|item| {
            matches!(&item.body, ItemBody::ToolCall {name, ..} if name == "bash")
                && !item.is_terminal()
        })
        .map(|item| item.id.clone())
        .collect();
    for id in ids {
        state.apply(&frame(
            state.seq.0 + 1,
            extended(
                "fixture.bash",
                "promote",
                bingo_sdk::slot::with(
                    json!({}),
                    [bingo_sdk::slot::Slot::Row {
                        item: id,
                        mark: None,
                        title: None,
                        badge: None,
                        caption: None,
                        body: None,
                        actions: vec![bingo_sdk::ActionItem {
                            meaning: None,
                            label: "background".into(),
                            key: Some('b'),
                            action: bingo_sdk::Action {
                                name: "bash.promote".into(),
                                args: json!(null),
                            },
                        }],
                    }],
                ),
            ),
        ));
    }
}
