use super::*;

pub(super) fn new(capture: &Capture) -> Scene {
    let now = capture.now();
    let clock = Arc::new(Mutex::new(Now {
        wall: jiff::Timestamp::from_millisecond(CLOCK_ORIGIN_MS).unwrap(),
        ..now
    }));
    let session = Arc::new(TestSession::default());
    let home = tempfile::tempdir().unwrap();
    let (replies, waiting) = mpsc::channel(16);
    let mut state = fixture::state();
    state.summary.choice.model = Some("opus-5.5".into());
    state.summary.provider = Some("anthropic".into());
    state.summary.effort = Some(bingo_sdk::Effort::High);
    let mut run = tests::idle_in(state, session.clone(), now.instant);
    let source = clock.clone();
    run.clock = Arc::new(move || {
        let now = source.lock().unwrap();
        (now.instant, now.wall)
    });
    run.clipboard_image = Arc::new(|| Some(bingo_pictures::testing::png_bytes(2, 2)));
    run.data_dir = home.path().into();
    run.replies = replies;
    run.reply(
        Reply::Catalogue("tools".into(), story::catalogue()),
        &mut None,
    );
    let host = bingo_sdk::testing::TestHost::default();
    let recorded = session.clone();
    host.on_withdraw(move |_, intent, _| {
        let input = recorded
            .submissions()
            .into_iter()
            .find(|(id, _)| id == &intent)
            .unwrap()
            .1;
        async move { answers::withdrawn(&input) }
    });
    run.host = host.handle();
    Scene {
        run,
        session,
        host,
        clock,
        replies: waiting,
        screen: Recorder::default(),
        _home: home,
        timeline: vec![],
        sent: 0,
        interrupted: 0,
        revision: 0,
        name: capture.entry.file.clone(),
        turns: turns(&capture.entry.file),
    }
}

pub(super) async fn setup(scene: &mut Scene) {
    let mut past = scene.run.session.tree.root().clone();
    let previous = story::turns(if scene.name.starts_with("image") {
        "done"
    } else {
        "live"
    });
    let count = if scene.name.starts_with("image") {
        2
    } else {
        1
    };
    for (n, turn) in previous.iter().take(count).enumerate() {
        story::fold_turn(&mut past, n, turn);
    }
    scene.run.session = Attached::new(past, SessionHandle(scene.session.clone()));
    scene.emit(Event::ConfigChanged {
        config: bingo_sdk::ConfigView {
            kernel: json!({"thinking":"high"}),
            plugins: Default::default(),
        },
    });
    scene.emit(fixture::permission_view(
        if scene.name.starts_with("live") || scene.name.starts_with("steer") {
            "acceptEdits"
        } else {
            "default"
        },
    ));
    if scene.name.starts_with("live")
        || scene.name.starts_with("stop")
        || scene.name.starts_with("steer")
    {
        let turn = scene.turns[0].clone();
        timeline::start(
            scene,
            &turn,
            1,
            IntentId::from_raw("before-attachment"),
            &Input::text(turn["prompt"].as_str().unwrap(), crate::origin()),
        );
        timeline::advance(scene, 0).await;
    }
    if scene.name.starts_with("steer") {
        scene.text("测试名字用英文", false).await;
        scene
            .key(fixture::key(crossterm::event::KeyCode::Enter))
            .await;
    }
    if scene.name.starts_with("stop") || scene.name.starts_with("steer") {
        scene.text(QUEUED, false).await;
        scene
            .key(fixture::key(crossterm::event::KeyCode::Tab))
            .await;
    }
    if scene.name.starts_with("stop") {
        scene.timeline.push((1800, timeline::Answer::Escape));
    }
    if scene.name.starts_with("shell") {
        scene.text("!git status --short", false).await;
        scene
            .key(fixture::key(crossterm::event::KeyCode::Enter))
            .await;
    }
    if scene.name.starts_with("image") {
        scene.key(fixture::ctrl('v')).await;
        scene.text("这个报错什么意思", false).await;
    }
    scene.timeline.sort_by_key(|(at, _)| *at);
}

fn turns(name: &str) -> Vec<serde_json::Value> {
    let extra: serde_json::Value =
        serde_json::from_str(include_str!("../../test_support/ember/acting-turns.json")).unwrap();
    if name.starts_with("stop") {
        let mut turns = vec![story::turns("stop")[1].clone()];
        if name == "stop-resent" {
            turns.extend(extra[name].as_array().unwrap().clone());
        }
        return turns;
    }
    if name.starts_with("steer") {
        return vec![story::turns("steer-landed")[1].clone()];
    }
    extra[name].as_array().cloned().unwrap_or_default()
}
