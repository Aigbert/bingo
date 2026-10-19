//! Sheet scenes press their capture's keys into a run whose host is a kernel
//! stand-in holding the owners: every sheet on screen is one an owner drew.
use super::*;
use crossterm::event::{KeyCode, KeyEvent};

mod asks;
mod builtins;
mod kernel;
mod memories;
mod schedules;
mod world;

struct Stage {
    scene: Scene,
    kernel: kernel::Kernel,
    calls: usize,
    /// What the provider streams for the prompts it was given, by scene time.
    provider: Vec<(i64, Event)>,
    /// The ask an ask scene holds open until a key answers it.
    ask: Option<asks::Asking>,
}

/// The frame the capture's keys leave on screen at its moment.
pub(crate) async fn draw(capture: &Capture) -> ratatui::buffer::Buffer {
    let mut stage = open(capture).await;
    for step in &capture.entry.after {
        if let Some(wait) = step.as_i64() {
            stage.wait(wait).await;
        } else if let Some(text) = step.as_array() {
            for ch in text[0].as_str().unwrap().chars() {
                stage.press(fixture::typed(ch)).await;
            }
        } else if let Some(key) = step.as_str().filter(|s| !s.starts_with("shot")) {
            stage.press(press(key)).await;
        }
    }
    stage.at(capture.entry.moment_ms).await;
    let size = capture.size();
    let now = stage.scene.run.now();
    let run = &mut stage.scene.run;
    let backend = fixture::drawn(size.width, size.height, &run.session.tree, &mut run.ui, now);
    backend.buffer().clone()
}

/// The world the capture starts from, with its sheet already opened by the
/// key or line that opens it.
async fn open(capture: &Capture) -> Stage {
    if asks::is_ask(&capture.entry.file) {
        return asks::open(capture).await;
    }
    let mut scene = setup::new(capture);
    scene.screen.size = Some((capture.size().width, capture.size().height));
    let mut state = world_state();
    if capture.entry.scene == "mention" {
        state.summary.cwd = repository(scene._home.path());
    } else {
        located(&mut state, scene._home.path());
    }
    scene.run.session = Attached::new(state.clone(), SessionHandle(scene.session.clone()));
    let kernel = kernel::Kernel::new(scene.host.clone(), state);
    let switched = world::Switched::default();
    world::script(&kernel, &switched);
    builtins::register(&kernel, &switched);
    schedules::register(&kernel);
    let env = bingo_sdk::Env::rooted(scene._home.path());
    memories::register(&kernel, env.home.clone());
    kernel.register(&bingo_mod_models::Models, &env);
    kernel.register(&bingo_mod_palette::Palette, &env);
    kernel.register(&bingo_tasks::TasksPlugin::default(), &env);
    kernel.register(&bingo_mod_plugins::Plugins, &env);
    kernel.register(&bingo_mod_sessions::Sessions, &env);
    kernel.register(&bingo_mod_schedule::SchedulePanel::default(), &env);
    kernel.register(&bingo_mod_status::Status::default(), &env);
    kernel.register(&bingo_mod_memory::MemoryPanel, &env);
    let mut stage = Stage {
        scene,
        kernel,
        calls: 0,
        provider: vec![],
        ask: None,
    };
    stage.scene.set_time(-OPENING_MS);
    super::catalogues::fetch_all(&mut stage.scene.run).await;
    stage.kernel.broadcast(config());
    stage.settle().await;
    let name = capture.entry.file.clone();
    let typed = match capture.entry.scene.as_str() {
        "palette" => Some("/"),
        "skills" => Some("/re"),
        "mention" => Some("看一下 @queue"),
        _ => None,
    };
    if let Some(typed) = typed {
        for ch in typed.chars() {
            stage.hit(fixture::typed(ch)).await;
        }
        stage.at(0).await;
        return stage;
    }
    match name.split('-').next().unwrap() {
        "keys" => stage.hit(fixture::typed('?')).await,
        "tasks" => stage.hit(fixture::ctrl('t')).await,
        "rewind" => {
            stage.hit(press("esc")).await;
            stage.hit(press("esc")).await;
        }
        sheet => {
            stage.scene.run.run_line(format!("/{sheet}"));
            stage.settle().await;
        }
    }
    stage.at(0).await;
    stage
}

/// The prototype's files under a fresh repository, with what git says of
/// them: `queue.rs` changed since the commit, the test and the ADR new.
fn repository(home: &std::path::Path) -> String {
    let root = home.join("bingo-improve");
    for row in crate::test_support::ember::world::rows("files") {
        let path = root.join(row["p"].as_str().unwrap());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        if row["st"] != "new" {
            std::fs::write(&path, "//! it\n").unwrap();
        }
    }
    let git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .args([
                "-c",
                "user.name=bingo",
                "-c",
                "user.email=bingo@example.test",
            ])
            .args(args)
            .current_dir(&root)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    };
    git(&["init", "-q"]);
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "world"]);
    for row in crate::test_support::ember::world::rows("files") {
        if row["st"].is_string() {
            let path = root.join(row["p"].as_str().unwrap());
            std::fs::write(path, "//! changed\n").unwrap();
        }
    }
    root.to_string_lossy().into_owned()
}

/// The prototype's `HERE`: the repository under home, on `dev` with three
/// files changed, as bingo-context's location slot says it.
fn located(state: &mut SessionState, home: &std::path::Path) {
    state.summary.cwd = home.join("bingo-improve").to_string_lossy().into_owned();
    let location = bingo_sdk::slot::Slot::Location {
        cwd: state.summary.cwd.clone(),
        branch: Some("dev".into()),
        changed: Some(3),
    };
    let event = Event::Extension {
        plugin: "_bingo.context".into(),
        kind: "location".into(),
        payload: bingo_sdk::slot::with(json!({}), [location]),
    };
    state.apply(&fixture::frame(state.seq.0 + 1, event));
}

/// A line that names one of the world's skills: bingo-skills turns it into
/// a prompt, so the provider answers it.
fn skill(input: &Input) -> bool {
    let Input::Text { text, .. } = input else {
        return false;
    };
    let name = text.strip_prefix('/').unwrap_or_default();
    crate::test_support::ember::world::rows("commands")
        .iter()
        .any(|row| row["kind"] == "skill" && row["n"] == name)
}

/// The opening key or line lands this long before the capture's clock starts,
/// so the sheet has finished growing when the capture's first moment is drawn.
const OPENING_MS: i64 = 1000;

/// The session every sheet opens over: the finished turn, on the model the
/// prototype's chat is using, with what its two turns spent and filled.
fn world_state() -> SessionState {
    let (mut state, _) = story::scene("done");
    let mut summary = state.summary.clone();
    summary.model = Some("opus-5.5".into());
    summary.provider = Some("anthropic".into());
    summary.effort = Some(bingo_sdk::Effort::High);
    summary.title = Some("steer 吞掉排队消息".into());
    summary.usage = world::SPENT;
    let context = Event::ContextChanged {
        context: Some(world::CONTEXT),
    };
    for event in [Event::SessionUpdated { summary }, context, world::tasks()] {
        state.apply(&fixture::frame(state.seq.0 + 1, event));
    }
    state
}

/// One config, as the core publishes it: the permission policy's view and
/// the session's thinking level together.
fn config() -> Event {
    let mut event = fixture::permission_view("default");
    if let Event::ConfigChanged { config } = &mut event {
        config.kernel["thinking"] = json!("high");
    }
    event
}

fn press(name: &str) -> KeyEvent {
    match name {
        "down" => fixture::key(KeyCode::Down),
        "up" => fixture::key(KeyCode::Up),
        "left" => fixture::key(KeyCode::Left),
        "right" => fixture::key(KeyCode::Right),
        "space" => fixture::typed(' '),
        "backspace" => fixture::key(KeyCode::Backspace),
        "ctrl+t" => fixture::ctrl('t'),
        name => timeline::key(name),
    }
}

impl Stage {
    /// A key, then what it set going, then the 60 ms the capture's driver
    /// leaves after every key.
    async fn press(&mut self, key: KeyEvent) {
        self.hit(key).await;
        self.wait(60).await;
    }

    async fn hit(&mut self, key: KeyEvent) {
        self.scene.run.terminal_event(Term::Key(key));
        self.settle().await;
    }

    async fn wait(&mut self, ms: i64) {
        self.at(self.scene.elapsed() + ms).await;
    }

    async fn at(&mut self, ms: i64) {
        self.provider.sort_by_key(|(at, _)| *at);
        while self.provider.first().is_some_and(|(at, _)| *at <= ms) {
            let (at, event) = self.provider.remove(0);
            self.scene.set_time(at);
            self.kernel.broadcast(event);
            self.settle().await;
        }
        self.scene.set_time(ms);
        self.settle().await;
    }

    /// Runs every task the scene set going until nothing more happens: the
    /// surface's submissions reach the kernel, the kernel's events and the
    /// owners' drawings reach the surface, and the surface paints.
    async fn settle(&mut self) {
        let mut quiet = 0;
        while quiet < 5 {
            let moved = self.forward() | self.deliver() | self.answer() | self.resolve();
            self.paint();
            for _ in 0..16 {
                tokio::task::yield_now().await;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
            quiet = if moved || !self.kernel.idle() {
                0
            } else {
                quiet + 1
            };
        }
    }

    fn forward(&mut self) -> bool {
        let submissions = self.scene.session.submissions();
        let fresh: Vec<_> = submissions.into_iter().skip(self.scene.sent).collect();
        self.scene.sent += fresh.len();
        let moved = !fresh.is_empty();
        for (intent, input) in fresh {
            if skill(&input) {
                self.prompt(intent, &input);
            } else {
                self.kernel.submit(intent, input);
            }
        }
        for (intent, input) in self.kernel.prompted() {
            self.prompt(intent, &input);
        }
        moved
    }

    /// The prototype answers a skill's line with the skill's script and
    /// every other prompt it has no script for with its echo: the turn
    /// starts now and its items stream from here.
    fn prompt(&mut self, intent: IntentId, input: &Input) {
        let Input::Text { text, .. } = input else {
            return;
        };
        let now = self.scene.run.now().wall;
        let script = if skill(input) {
            "skillScript"
        } else {
            "echoScript"
        };
        for event in answers::sent(&format!("script-{script}"), intent, input, now) {
            self.kernel.broadcast(event);
        }
        let timeline = story::script_timeline(script, text, now);
        let provider = timeline
            .into_iter()
            .map(|(at, event)| (at - CLOCK_ORIGIN_MS, event));
        self.provider.extend(provider);
    }

    /// The kernel's events, then the drawings the owners said changed.
    fn deliver(&mut self) -> bool {
        let mut events = self.kernel.take();
        let calls = self.kernel.host().calls();
        for call in calls.iter().skip(self.calls) {
            if let bingo_sdk::testing::HostCall::Invalidate { requests, .. } = call {
                events.push(Event::Invalidated {
                    requests: requests.clone(),
                });
            }
        }
        self.calls = calls.len();
        let announced = self.kernel.announced();
        let moved = !events.is_empty() || !announced.is_empty();
        self.scene.emit_all(events);
        for event in announced {
            if let bingo_sdk::GatewayEvent::CatalogChanged { kind } = event {
                super::catalogues::changed(&mut self.scene.run, kind);
            }
        }
        moved
    }

    fn answer(&mut self) -> bool {
        let mut moved = false;
        while let Ok(reply) = self.scene.replies.try_recv() {
            self.scene.run.reply(reply, &mut None);
            moved = true;
        }
        moved
    }

    fn paint(&mut self) {
        let now = self.scene.run.now();
        self.scene
            .run
            .paint(&mut self.scene.screen, Wake::Frame, now)
            .unwrap();
    }
}
