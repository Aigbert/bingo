use super::*;
use crossterm::event::{KeyCode, KeyEvent};

pub(super) enum Answer {
    Frame(Box<Event>),
    Steer(String, String),
    Escape,
}

pub(super) fn key(name: &str) -> KeyEvent {
    match name {
        "enter" => fixture::key(KeyCode::Enter),
        "tab" => fixture::key(KeyCode::Tab),
        "esc" => fixture::key(KeyCode::Esc),
        "up" => fixture::key(KeyCode::Up),
        "ctrl+v" => fixture::ctrl('v'),
        _ => panic!("unported acting key {name}"),
    }
}

pub(super) async fn advance(scene: &mut Scene, to: i64) {
    scene.timeline.sort_by_key(|(at, _)| *at);
    while scene.timeline.first().is_some_and(|(at, _)| *at <= to) {
        let (at, answer) = scene.timeline.remove(0);
        scene.set_time(at);
        match answer {
            Answer::Frame(event) => {
                let promote = match event.as_ref() {
                    Event::ItemStarted { item } if matches!(&item.body, ItemBody::ToolCall { name, .. } if name == "bash") => {
                        Some(item.id.clone())
                    }
                    _ => None,
                };
                scene.emit(*event);
                if let Some(id) = promote {
                    timeline::promote(scene, id);
                }
            }
            Answer::Steer(id, prompt) => timeline::land(scene, &id, &prompt),
            Answer::Escape => scene.key(key("esc")).await,
        }
    }
    scene.set_time(to);
}

pub(super) fn start(scene: &mut Scene, turn: &Value, n: usize, intent: IntentId, input: &Input) {
    let now = scene.elapsed();
    scene.set_time(turn["t0"].as_i64().unwrap() - CLOCK_ORIGIN_MS);
    scene.emit_all(answers::sent(
        &format!("ember-{n}"),
        intent,
        input,
        scene.run.now().wall,
    ));
    scene.set_time(now);
    scene.timeline.extend(
        story::provider_events(n, turn)
            .into_iter()
            .map(|(at, event)| (at - CLOCK_ORIGIN_MS, Answer::Frame(Box::new(event)))),
    );
    for (index, data) in turn["items"].as_array().unwrap().iter().enumerate() {
        if data["k"] == "you" {
            scene.timeline.push((
                data["at"].as_i64().unwrap() - CLOCK_ORIGIN_MS,
                Answer::Steer(
                    format!("ember-{n}-{index}"),
                    data["text"].as_str().unwrap().into(),
                ),
            ));
        }
    }
}

pub(super) fn submitted(scene: &mut Scene) {
    for (intent, input) in scene.session.submissions().into_iter().skip(scene.sent) {
        scene.sent += 1;
        let Input::Text { text, .. } = &input else {
            panic!("acting only sends text");
        };
        if text.starts_with('!') {
            timeline::shell(scene, intent, text);
        } else if scene.run.session.tree.viewed().busy() {
            scene.revision += 1;
            scene.emit_all(answers::queued(
                scene.run.session.tree.viewed(),
                intent,
                &input,
                scene.revision,
            ));
        } else {
            let turn = scene.turns.last().unwrap().clone();
            let expected = turn["prompt"].as_str().unwrap();
            // The composer token names the Image part, not a word in the prototype's prompt.
            assert_eq!(text.trim_start_matches("[image 1] "), expected);
            timeline::start(scene, &turn, 2, intent, &input);
        }
    }
}

pub(super) fn shell(scene: &mut Scene, intent: IntentId, text: &str) {
    let turn = scene
        .turns
        .iter()
        .find(|turn| turn["prompt"] == text)
        .unwrap();
    let at = turn["endAt"].as_f64().unwrap() as i64;
    let output = turn["items"][0]["out"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    let event = answers::shell(
        text,
        &output,
        &scene.run.session.tree.root().summary.cwd,
        jiff::Timestamp::from_millisecond(at).unwrap(),
        &format!("shell-{}", scene.sent),
        intent.clone(),
    );
    let Event::ItemCompleted { ref item } = event else {
        unreachable!()
    };
    let ack = Event::IntentAck {
        intent,
        outcome: IntentOutcome::Applied {
            result: json!({"item":item.id}),
        },
    };
    scene
        .timeline
        .push((at - CLOCK_ORIGIN_MS, Answer::Frame(Box::new(event))));
    scene
        .timeline
        .push((at - CLOCK_ORIGIN_MS, Answer::Frame(Box::new(ack))));
}

pub(super) fn land(scene: &mut Scene, id: &str, prompt: &str) {
    let entry = scene
        .run
        .session
        .tree
        .viewed()
        .queue
        .iter()
        .find(|q| q.steerable)
        .unwrap()
        .clone();
    let input = scene
        .session
        .submissions()
        .into_iter()
        .find(|(intent, _)| intent == &entry.intent)
        .unwrap()
        .1;
    assert!(matches!(&input, Input::Text { text, .. } if text == prompt));
    scene.revision += 1;
    scene.emit_all(answers::steered(
        scene.run.session.tree.viewed(),
        &entry.intent,
        &input,
        id,
        scene.run.now().wall,
        scene.revision,
    ));
}

pub(super) fn promote(scene: &mut Scene, id: bingo_sdk::ItemId) {
    // This is the tool plugin's declared action, absent from the turn data.
    scene.emit(fixture::extended(
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
                        args: Value::Null,
                    },
                }],
            }],
        ),
    ));
}
