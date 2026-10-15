use super::*;
use crate::{doubles::TestSession, test_support::*};
use bingo_sdk::{
    Command, CommandContext, CommandOutcome, Contribution, Env, HookContext, Interceptor, Next,
    Plugin, Registrar, catalogue, render, testing::TestHost,
};
use crossterm::event::KeyCode;
use serde_json::json;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

struct ScheduleFixture {
    home: tempfile::TempDir,
    plugin: bingo_schedule::SchedulePlugin,
    command: Arc<dyn Command>,
    host: TestHost,
    signals: Arc<Mutex<Vec<Event>>>,
    open: Arc<dyn Interceptor>,
    draw: Arc<dyn Interceptor>,
    handler: Arc<dyn Command>,
    session: Arc<TestSession>,
    run: Run,
}
async fn owner(env: &Env) -> (bingo_schedule::SchedulePlugin, TestHost, Arc<dyn Command>) {
    let plugin = bingo_schedule::SchedulePlugin::default();
    let mut registrar = Registrar::new(
        plugin.manifest().id,
        json!({}),
        env.clone(),
        Default::default(),
    );
    plugin.register(&mut registrar).unwrap();
    let contributions = registrar.into_contributions();
    let command = contributions
        .iter()
        .find_map(|c| match c {
            Contribution::Command(c) if c.spec().name == "schedule" => Some(c.clone()),
            _ => None,
        })
        .unwrap();
    let services: HashMap<_, _> = contributions
        .into_iter()
        .filter_map(|c| match c {
            Contribution::Service { key, value, .. } => Some((key, value)),
            _ => None,
        })
        .collect();
    let host = TestHost::default();
    host.on_service_any(move |key| services.get(&key).cloned());
    host.on_sessions(|_| async { Ok(vec![]) });
    delivery(&host);
    plugin.start(host.handle()).await.unwrap();
    (plugin, host, command)
}
fn delivery(host: &TestHost) {
    let sender = Arc::new(Mutex::new(
        None::<tokio::sync::mpsc::UnboundedSender<bingo_sdk::Frame>>,
    ));
    let opening = sender.clone();
    host.on_open(move |_, _, _| {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        *opening.lock().unwrap() = Some(tx);
        async move {
            Ok(Attachment {
                session: SessionId::from_raw("ses_fired"),
                snapshot: state(),
                history: None,
                events: Box::pin(futures::stream::once(
                    async move { rx.recv().await.unwrap() },
                )),
                handle: SessionHandle(Arc::new(TestSession::default())),
            })
        }
    });
    host.on_deliver(move |_, intent, _, _| {
        sender
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .send(frame(
                1,
                Event::IntentAck {
                    intent,
                    outcome: IntentOutcome::Queued { position: 1 },
                },
            ))
            .unwrap();
        async { Ok(()) }
    });
}
impl ScheduleFixture {
    async fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let env = Env::rooted(home.path());
        let (plugin, host, command) = owner(&env).await;
        let signals = Arc::new(Mutex::new(Vec::new()));
        let collected = signals.clone();
        host.on_signal(move |_, plugin, kind, payload| {
            collected.lock().unwrap().push(Event::Signal {
                generation: None,
                plugin,
                kind,
                payload,
            });
            async { Ok(()) }
        });
        let mut registrar = Registrar::new(
            bingo_mod_schedule::SchedulePanel::default().manifest().id,
            json!({}),
            env.clone(),
            Default::default(),
        );
        bingo_mod_schedule::SchedulePanel::default()
            .register(&mut registrar)
            .unwrap();
        let contributions = registrar.into_contributions();
        let link = |id| {
            contributions
                .iter()
                .find_map(|c| match c {
                    Contribution::Interceptor { link, .. } if link.id() == id => Some(link.clone()),
                    _ => None,
                })
                .unwrap()
        };
        let handler = contributions
            .iter()
            .find_map(|c| match c {
                Contribution::Command(command) => Some(command.clone()),
                _ => None,
            })
            .unwrap();
        let mut state = state();
        state.summary.cwd = home.path().display().to_string();
        state.config.kernel["render"] = json!({"Pane":[{}]});
        let session = Arc::new(TestSession::default());
        let mut run = tests::idle_in(state, session.clone(), still(scene().1).instant);
        run.data_dir = env.data_dir.clone();
        run.host = host.handle();
        Self {
            home,
            plugin,
            command,
            host,
            signals,
            open: link("schedules.open"),
            draw: link("bingo.mod.schedule"),
            handler,
            session,
            run,
        }
    }
    fn cx(&self) -> HookContext {
        HookContext {
            session: self.run.session.tree.view().clone(),
            turn: None,
            item: None,
            cwd: self.home.path().into(),
            provider: None,
            model: None,
            host: self.host.handle(),
        }
    }
    fn fold(&mut self) {
        for event in self.signals.lock().unwrap().drain(..) {
            let seq = self.run.session.tree.root().seq.next().0;
            self.run.session.tree.fold.apply(&frame(seq, event));
        }
    }
    async fn open(&mut self) {
        let outcome = self
            .open
            .chain(
                catalogue::COMMAND_RUN,
                &self.cx(),
                json!({"command":"schedule","args":"","surface":"terminal"}),
                Next::new(|_| {
                    Box::pin(async { Ok(json!({"kind":"applied","message":"fallback"})) })
                }),
            )
            .await
            .unwrap();
        self.fold();
        let CommandOutcome::Surface { action, fallback } = serde_json::from_value(outcome).unwrap()
        else {
            panic!("surface")
        };
        assert!(fallback.is_none());
        surface::action(&mut self.run, action);
    }
    async fn line(&self, line: &str) -> CommandOutcome {
        let cx = self.cx();
        render::under_input(
            true,
            self.command.run(
                line,
                &CommandContext {
                    session: cx.session,
                    cwd: cx.cwd,
                    host: cx.host,
                    cancel: Default::default(),
                },
            ),
        )
        .await
        .unwrap()
    }
    async fn paint(&mut self, width: u16, height: u16) -> String {
        self.fold();
        let now = still(scene().1);
        draw_tree(width, height, &self.run.session.tree, &mut self.run.ui, now);
        let key = self
            .run
            .ui
            .shown
            .pulls
            .iter()
            .find(|key| key.request.component == render::Component::Pane)
            .unwrap()
            .clone();
        let value = self
            .draw
            .chain(
                catalogue::UI_RENDER,
                &self.cx(),
                serde_json::to_value(&key.request).unwrap(),
                Next::new(|v| Box::pin(async { Ok(v) })),
            )
            .await
            .unwrap();
        let view: View = serde_json::from_value(value).unwrap();
        bingo_sdk::view::answer_ok(
            render::Component::Pane,
            &key.request.props,
            "bingo.mod.schedule",
            None,
            &view,
        )
        .unwrap();
        self.run.ui.placed.pulled.request(&key);
        self.run.ui.placed.pulled.answer(key, view);
        draw_tree(width, height, &self.run.session.tree, &mut self.run.ui, now)
    }
    async fn press(&mut self, key: crossterm::event::KeyEvent) {
        let count = self.session.submissions().len();
        self.run.terminal_event(Term::Key(key));
        assert_eq!(
            self.session.submissions().len(),
            count + 1,
            "a fresh key action"
        );
        let submissions = self.session.submissions();
        let (intent, Input::Action { action, .. }) = submissions.last().unwrap() else {
            panic!("pane action")
        };
        let cx = self.cx();
        let outcome = render::under_press(
            true,
            self.handler.run(
                &action.args.to_string(),
                &CommandContext {
                    session: cx.session,
                    cwd: cx.cwd,
                    host: cx.host,
                    cancel: Default::default(),
                },
            ),
        )
        .await
        .unwrap();
        self.fold();
        let result = match outcome {
            CommandOutcome::Handled { handled } => json!({"handled":handled}),
            CommandOutcome::Applied { message } => json!({"message":message}),
            CommandOutcome::Surface { action, .. } => {
                json!({"surface":action,"drawnBy":"bingo.mod.schedule"})
            }
            _ => panic!("panel receipt"),
        };
        self.run.ack(intent, &IntentOutcome::Applied { result });
        if let Some((intent, Input::Text { text, .. })) = self.session.submissions().get(count + 1)
        {
            self.line(text.strip_prefix("/schedule ").unwrap()).await;
            self.run
                .ack(intent, &IntentOutcome::Applied { result: json!({}) });
        }
    }
}

#[tokio::test]
async fn panel_keys_change_the_same_table_and_deliver_a_wake() {
    let mut f = ScheduleFixture::new().await;
    f.line("add every 2h -- check build").await;
    f.open().await;
    assert!(f.paint(108, 30).await.contains("check build"));
    f.press(typed(' ')).await;
    assert!(f.paint(80, 24).await.contains("off"));
    let CommandOutcome::View { view } = f.line("").await else {
        panic!("table")
    };
    assert!(view.fold().contains("no"));
    f.press(typed(' ')).await;
    f.paint(80, 24).await;
    f.press(typed('r')).await;
    assert!(f.host.calls().iter().any(|call| matches!(
        call,
        bingo_sdk::testing::HostCall::Deliver {
            delivery: bingo_sdk::Delivery::Wake,
            ..
        }
    )));
    f.paint(108, 30).await;
    f.press(typed('d')).await;
    assert!(f.paint(108, 30).await.contains("its session stays"));
    f.press(key(KeyCode::Enter)).await;
    assert!(f.paint(80, 24).await.contains("no schedules yet"));
    f.plugin.stop().await.unwrap();
}
