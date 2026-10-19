//! Edge fixtures enter through terminal keys and folded frames.
use super::*;
use crate::test_support::ember::{Band, assert_band};
use bingo_sdk::{HostApi, Tone};

fn data(name: &str) -> Value {
    let all: Value =
        serde_json::from_str(include_str!("../../test_support/ember/edges.json")).unwrap();
    all[name].clone()
}

fn fold(scene: &mut Scene, turns: &[Value]) {
    let mut state = scene.run.session.tree.root().clone();
    for (n, turn) in turns.iter().enumerate() {
        story::fold_turn(&mut state, n, turn);
    }
    scene.run.session = Attached::new(state, SessionHandle(scene.session.clone()));
}

/// The prototype's RECENTS, the sessions the welcome lists for this cwd.
pub(super) fn recents() -> Vec<bingo_sdk::SessionSummary> {
    [
        ("steer 吞掉排队消息", 720),
        ("给 total 加标签", 7200),
        ("bingo-site 文档页", 86400),
    ]
    .into_iter()
    .enumerate()
    .map(|(n, (title, seconds))| {
        let mut summary = fixture::summary();
        summary.id = SessionId::from_raw(format!("recent-{n}"));
        summary.title = Some(title.into());
        summary.updated_at = jiff::Timestamp::from_millisecond(CLOCK_ORIGIN_MS).unwrap()
            - jiff::SignedDuration::from_secs(seconds);
        summary
    })
    .collect()
}

async fn welcome(scene: &mut Scene, capture: &Capture) {
    scene.run.ui.welcome.started_at_ms = Some(CLOCK_ORIGIN_MS);
    let recent = recents();
    scene.host.on_render(move |_, mut request| {
        let recent = recent.clone();
        async move {
            request.props["recent"] = json!(recent);
            request.props["branch"] = json!("dev");
            request.props["changed"] = json!(3);
            Ok(request.engine())
        }
    });
    scene.emit(Event::ConfigChanged {
        config: bingo_sdk::ConfigView {
            kernel: json!({"render":{"Welcome":[{}]}}),
            ..Default::default()
        },
    });
    follow(scene, capture).await;
}

/// The capture's `after` steps, typed and waited through the terminal.
async fn follow(scene: &mut Scene, capture: &Capture) {
    for step in &capture.entry.after {
        if let Some(wait) = step.as_i64() {
            timeline::advance(scene, scene.elapsed() + wait).await;
        } else if let Some(text) = step.as_array() {
            scene.text(text[0].as_str().unwrap(), true).await;
        } else if let Some(key) = step.as_str().filter(|s| !s.starts_with("shot")) {
            scene.key(timeline::key(key)).await;
            timeline::advance(scene, scene.elapsed() + 60).await;
        }
        pull(scene, capture).await;
    }
}

/// The second prompt is typed and sent; the provider refuses it as signed out.
async fn signed_out(scene: &mut Scene, capture: &Capture) {
    let mut summary = scene.run.session.tree.root().summary.clone();
    summary.provider = Some("anthropic".into());
    scene.emit(Event::SessionUpdated { summary });
    scene.run.ui.catalogs.commands = crate::commands::entries(vec![command("login")]);
    let prompt = scene.turns[1]["prompt"].as_str().unwrap().to_owned();
    scene.text(&prompt, false).await;
    scene.key(timeline::key("enter")).await;
    follow(scene, capture).await;
    timeline::advance(scene, capture.entry.moment_ms).await;
}

/// The CI turn set a wake that is still pending; it fires on its own, with
/// the model's own words and nobody at the keyboard.
async fn woken(scene: &mut Scene, capture: &Capture) {
    let turns = scene.turns.clone();
    fold(scene, &turns[..2]);
    let woken = &turns[2];
    let fires = woken["t0"].as_i64().unwrap();
    let at = jiff::Timestamp::from_millisecond(fires).unwrap();
    scene.emit(fixture::pending_wake(&at.to_string()));
    let input = Input::text(
        woken["prompt"].as_str().unwrap(),
        bingo_sdk::Origin::surface("wake").voiced(bingo_sdk::Voice::Note),
    );
    let fired = std::iter::once(fixture::fired_wake())
        .chain(
            answers::sent("ember-2", bingo_sdk::IntentId::from_raw("wake"), &input, at)
                .into_iter()
                .filter(|event| !matches!(event, Event::IntentAck { .. })),
        )
        .map(|event| (fires, event))
        .chain(story::provider_events(2, woken));
    scene.timeline.extend(fired.map(|(at, event)| {
        (
            at - CLOCK_ORIGIN_MS,
            timeline::Answer::Frame(Box::new(event)),
        )
    }));
    follow(scene, capture).await;
    timeline::advance(scene, capture.entry.moment_ms).await;
}

fn command(name: &str) -> bingo_sdk::CommandSpec {
    bingo_sdk::CommandSpec {
        name: name.into(),
        aliases: vec![],
        hint: String::new(),
        args: bingo_sdk::ArgSpec::None,
        instant: false,
        family: "kernel".into(),
        opens: false,
        keys: vec![],
        hidden: false,
    }
}

async fn pull(scene: &mut Scene, capture: &Capture) {
    let size = capture.size();
    let now = scene.run.now();
    fixture::drawn(
        size.width,
        size.height,
        &scene.run.session.tree,
        &mut scene.run.ui,
        now,
    );
    let keys = scene.run.ui.shown.pulls.clone();
    for key in keys {
        let view = scene
            .host
            .render(&key.session, key.request.clone())
            .await
            .unwrap();
        scene.run.ui.placed.pulled.request(&key);
        scene.run.ui.placed.pulled.answer(key, view);
    }
}

pub(super) async fn draw(capture: &Capture) {
    let mut scene = setup::new(capture);
    let source = data(&capture.entry.file);
    scene.turns = source["log"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["kind"] == "turn")
        .cloned()
        .collect();
    let mut summary = scene.run.session.tree.root().summary.clone();
    summary.cwd = "~/bingo-improve".into();
    summary.title = source["title"].as_str().map(str::to_owned);
    scene.emit(Event::SessionUpdated { summary });
    scene.emit(fixture::permission_view(
        if capture.entry.file == "narrow" {
            "acceptEdits"
        } else {
            "default"
        },
    ));
    if capture.entry.scene == "welcome" {
        welcome(&mut scene, capture).await;
    } else {
        if capture.entry.scene == "background" {
            background_start(&mut scene).await;
        } else if capture.entry.scene == "signedout" {
            let turns = scene.turns.clone();
            fold(&mut scene, &turns[..1]);
            signed_out(&mut scene, capture).await;
        } else if capture.entry.scene == "wake" {
            woken(&mut scene, capture).await;
        } else {
            let turns = scene.turns.clone();
            fold(&mut scene, &turns);
        }
        apply_edges(&mut scene, capture, &source).await;
    }
    scene.set_time(capture.entry.moment_ms);
    pull(&mut scene, capture).await;
    let size = capture.size();
    let now = scene.run.now();
    let backend = fixture::drawn(
        size.width,
        size.height,
        &scene.run.session.tree,
        &mut scene.run.ui,
        now,
    );
    assert_band(capture, Band::Frame, backend.buffer());
}

async fn apply_edges(scene: &mut Scene, capture: &Capture, source: &Value) {
    if capture.entry.file == "narrow" {
        let id = scene
            .run
            .session
            .tree
            .root()
            .transcript
            .items()
            .iter()
            .find(|item| {
                matches!(&item.body, ItemBody::ToolCall { name, .. } if name == "bash")
                    && !item.status.is_terminal()
            })
            .unwrap()
            .id
            .clone();
        timeline::promote(scene, id);
    }
    if capture.entry.file.starts_with("retry") {
        let retry = &source["log"][1]["retry"];
        scene.set_time(retry["at"].as_i64().unwrap() - CLOCK_ORIGIN_MS);
        scene.emit(Event::TurnRetrying {
            turn: scene
                .run
                .session
                .tree
                .root()
                .turn
                .as_ref()
                .unwrap()
                .id
                .clone(),
            attempt: retry["n"].as_u64().unwrap() as u32,
            max: 10,
            delay_ms: (retry["until"].as_i64().unwrap() - retry["at"].as_i64().unwrap()) as u64,
            reason: "rate limited".into(),
            dropped: vec![],
        });
    }
    if capture.entry.scene == "context" {
        context(scene, capture).await;
    }
    if capture.entry.scene == "background" {
        background(scene, source);
    }
    if capture.entry.file == "exit-back" {
        scene.set_time(660);
        let attachment = bingo_sdk::Attachment {
            session: scene.run.session.tree.root_id().clone(),
            snapshot: scene.run.session.tree.root().clone(),
            history: None,
            handle: SessionHandle(scene.session.clone()),
            events: Box::pin(futures::stream::empty()),
        };
        scene
            .run
            .reply(Reply::Attached(Box::new(attachment)), &mut None);
    }
}

async fn context(scene: &mut Scene, capture: &Capture) {
    scene.emit(Event::ContextChanged {
        context: Some(bingo_sdk::ContextUsage {
            used: 86000,
            window: 100000,
            ..Default::default()
        }),
    });
    if capture.entry.file.ends_with("compacted") {
        scene.run.ui.catalogs.commands = crate::commands::entries(vec![command("compact")]);
        scene.text("/compact", false).await;
        scene
            .run
            .terminal_event(Term::Key(fixture::key(crossterm::event::KeyCode::Enter)));
        assert!(
            matches!(scene.session.submitted().last(), Some(Input::Text {text, ..}) if text == "/compact")
        );
        scene.sent = scene.session.submissions().len();
        scene.set_time(780);
        let mut item = fixture::item(
            "compact",
            bingo_sdk::ItemStatus::Completed,
            ItemBody::Compaction {
                summary: "summary".into(),
                before: 86000,
                after: 18000,
                replaced: 0,
                duration_ms: 0,
                window: Some(100000),
            },
        );
        item.turn = None;
        item.started_at = scene.run.now().wall;
        scene.emit(Event::ItemCompleted { item });
        scene.emit(Event::ContextChanged {
            context: Some(bingo_sdk::ContextUsage {
                used: 18000,
                window: 100000,
                ..Default::default()
            }),
        });
    }
}

async fn background_start(scene: &mut Scene) {
    let turns = scene.turns.clone();
    fold(scene, &turns[..1]);
    let turn = &turns[1];
    let mut live = turn.clone();
    live["state"] = json!("live");
    live["reply"] = Value::Null;
    live["items"][1]["st"] = json!("run");
    live["items"][1]["end"] = Value::Null;
    let mut state = scene.run.session.tree.root().clone();
    story::fold_turn(&mut state, 1, &live);
    scene.run.session = Attached::new(state, SessionHandle(scene.session.clone()));
    timeline::promote(scene, bingo_sdk::ItemId::from_raw("ember-1-1"));
    scene.run.terminal_event(Term::Key(fixture::ctrl('b')));
    assert!(
        matches!(scene.session.submitted().last(), Some(Input::Action {action, ..}) if action.name == "bash.promote")
    );
    scene.sent = scene.session.submissions().len();
    for (at, event) in story::provider_events(1, turn) {
        scene.set_time(at - CLOCK_ORIGIN_MS);
        scene.emit(event);
    }
}

fn background(scene: &mut Scene, source: &Value) {
    if !source["jobs"].as_array().unwrap().is_empty() {
        scene.emit(fixture::extended(
            "fixture.bash",
            "jobs",
            bingo_sdk::slot::with(
                json!({}),
                [bingo_sdk::slot::Slot::Status {
                    text: "1 shell".into(),
                    tone: Tone::Neutral,
                    until: None,
                }],
            ),
        ));
    } else {
        scene.set_time(11500);
        let mut item = fixture::user("job-finished", "model notice");
        item.turn = None;
        item.started_at = scene.run.now().wall;
        item.set_headline(bingo_sdk::Headline {
            tone: Tone::Good,
            parts: vec![
                "`cargo test --workspace` finished in the background".into(),
                "412 passed".into(),
            ],
        });
        scene.emit(Event::ItemCompleted { item });
        scene.emit(Event::Signal {
            generation: None,
            plugin: "fixture.bash".into(),
            kind: "job-finished".into(),
            payload: bingo_sdk::slot::with(
                json!({}),
                [bingo_sdk::slot::Slot::Toast {
                    parts: vec!["background job finished · 412 passed".into()],
                    tone: Tone::Good,
                    until: scene.run.now().wall + Duration::from_millis(2400),
                    alert: false,
                }],
            ),
        });
    }
}

/// The edge capture `name`, its keys typed through the terminal, compared
/// full frame.
pub(crate) fn full_frame(name: &str) {
    let spinners: &[(u16, u16)] = match name {
        "welcome-sent" => &[(25, 3)],
        "retry" | "retry-recovered" => &[(26, 3)],
        "narrow" => &[(17, 3)],
        "signedout-login" => &[(29, 3)],
        _ => &[],
    };
    let capture = Capture::load(name).spinners(spinners);
    capture.render(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(draw(&capture))
    });
}
