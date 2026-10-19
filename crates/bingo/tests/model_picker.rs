//! Model-site actions use only local providers and private credential answers.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::support;
use bingo_sdk::*;
use futures::StreamExt;
use serde_json::json;
use support::{LIMIT, Server, ack_for, create, ready, who};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};
const SCRIPT: &str = r#"{"responses":[{"steps":[{"text":"unused"}]}]}"#;
const SECRET: &str = "sk-dummy-picker-must-never-be-drawn-or-journaled";
async fn command(session: &mut Attachment, name: &str, args: serde_json::Value) -> IntentOutcome {
    let intent = IntentId::mint();
    let origin = Origin::surface("rpc").on(render::SurfaceName::Terminal);
    let input = match args.as_str() {
        Some(args) => Input::text(format!("/{name} {args}"), origin),
        None => Input::action(
            Action {
                name: name.into(),
                args,
            },
            origin,
        ),
    };
    session.handle.submit(intent.clone(), input);
    ack_for(session, &intent).await
}
async fn request(kernel: &impl HostApi, session: &mut Attachment) -> render::RenderRequest {
    let attached = kernel
        .open(
            SessionSelector::ById {
                id: session.session.clone(),
            },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    *session = attached;
    slot::placed(&session.snapshot)
        .into_iter()
        .find_map(|placed| match placed.slot {
            slot::Slot::Pane {
                view: slot::Shown::Pull { request },
                ..
            } if request.component == render::Component::ModelPicker => {
                Some(request.on(render::SurfaceName::Terminal))
            }
            _ => None,
        })
        .expect("picker pane")
}
fn press(id: &str, key: &str) -> serde_json::Value {
    json!({"handler":id,"event":"press","value":{"key":key}})
}
async fn paste(session: &mut Attachment, value: &str) -> InteractionId {
    tokio::time::timeout(LIMIT, async {
        loop {
            let frame = session
                .events
                .next()
                .await
                .expect("event stream ended while awaiting private paste");
            session.snapshot.apply(&frame);
            if let Event::InteractionOpened { interaction } = frame.event {
                assert!(matches!(
                    interaction.kind,
                    InteractionKind::Login {
                        flow: LoginFlow::Paste,
                        ..
                    }
                ));
                let id = interaction.id.clone();
                session.handle.answer(
                    IntentId::mint(),
                    interaction.id,
                    Answer::Text { text: value.into() },
                    Activation::Programmatic,
                );
                return id;
            }
        }
    })
    .await
    .unwrap()
}
async fn private_press(session: &mut Attachment, id: &str, key: &str, value: &str) {
    let intent = IntentId::mint();
    session.handle.submit(
        intent.clone(),
        Input::Action {
            origin: Some(Origin::surface("rpc").on(render::SurfaceName::Terminal)),
            action: Action {
                name: "bingo.mod.models:ui".into(),
                args: press(id, key),
            },
        },
    );
    paste(session, value).await;
    assert!(matches!(
        ack_for(session, &intent).await,
        IntentOutcome::Applied { .. }
    ));
}
async fn endpoint(id: &str) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":[{"id":id}]})))
        .mount(&server)
        .await;
    server
}
#[tokio::test(flavor = "multi_thread")]
async fn signin_key_is_absent_from_frames_and_the_journal() {
    let endpoint = endpoint("claude-fable-5").await;
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".bingo")).unwrap();
    std::fs::write(
        home.path().join(".bingo/settings.toml"),
        format!(
            "[update]\ncheck = false\n[anthropic]\nbaseUrl = \"{}\"\n",
            endpoint.uri()
        ),
    )
    .unwrap();
    let mut server = Server::spawn_at(home, SCRIPT, &[]);
    let kernel = ready(&mut server).await;
    let mut session = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    command(&mut session, "model", json!("")).await;
    let req = request(&kernel, &mut session).await;
    let id = req.request_id.clone();
    let model = kernel
        .catalog(CatalogKind::Models)
        .await
        .unwrap()
        .entries
        .into_iter()
        .find(|row| row.meta["provider"] == "anthropic" && row.meta["chat"] != false)
        .unwrap();
    command(
        &mut session,
        "bingo.mod.models:ui",
        json!({"handler":id,"event":"press","value":{"pick":format!("model:{}", model.id)}}),
    )
    .await;
    let req = request(&kernel, &mut session).await;
    let before = kernel.render(&session.session, req).await.unwrap();
    assert!(
        before.fold().contains("sign in to anthropic"),
        "{}",
        before.fold()
    );
    private_press(&mut session, &id, "enter", SECRET).await;
    let attached = reopen(&kernel, &session).await;
    assert!(!picker_open(&attached));
    assert!(
        !serde_json::to_string(&attached.snapshot)
            .unwrap()
            .contains(SECRET)
    );
    for file in walk(&server.sessions_dir()) {
        if file.extension().is_some_and(|ext| ext == "jsonl") {
            assert!(!std::fs::read_to_string(file).unwrap().contains(SECRET));
        }
    }
}

fn walk(path: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(path).unwrap().flatten() {
        if entry.path().is_dir() {
            files.extend(walk(&entry.path()));
        } else {
            files.push(entry.path());
        }
    }
    files
}
#[tokio::test(flavor = "multi_thread")]
async fn next_keys_cycle_pins_and_efforts_without_keeping() {
    let endpoint = endpoint("gpt-5").await;
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".bingo")).unwrap();
    std::fs::write(
        home.path().join(".bingo/settings.toml"),
        format!(
            r#"[update]
check = false
[openai]
baseUrl = "{}"
apiKey = "dummy-local-only"
"#,
            endpoint.uri()
        ),
    )
    .unwrap();
    let mut server = Server::spawn_at(home, SCRIPT, &[]);
    let kernel = ready(&mut server).await;
    let mut session = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    for id in ["fake/fake-1", "openai/gpt-5"] {
        let result = command(&mut session, "model", json!(format!("pin {id}"))).await;
        assert!(
            matches!(result, IntentOutcome::Applied { .. }),
            "{id}: {result:?}"
        );
    }
    let before = std::fs::read(server.cwd().join(".bingo/settings.toml")).unwrap();
    let specs = kernel
        .catalog(CatalogKind::Commands)
        .await
        .unwrap()
        .entries
        .into_iter()
        .filter_map(|e| serde_json::from_value(e.meta).ok())
        .collect::<Vec<CommandSpec>>();
    let keys = keys::bindings(&specs, &Default::default(), &[]);
    assert_eq!(keys.commands["alt+m"], "/model next");
    assert_eq!(keys.commands["alt+t"], "/think next");
    assert!(!keys.commands.contains_key("ctrl+p"));
    // The chords belong to `/model` and `/think`; the catalogue lists each once.
    let names: Vec<_> = specs.iter().map(|spec| spec.name.as_str()).collect();
    assert!(
        !names.contains(&"model next") && !names.contains(&"think next"),
        "{names:?}"
    );
    for expected in ["gpt-5", "fake-1"] {
        let result = command(&mut session, "model", json!("next")).await;
        assert!(
            matches!(result, IntentOutcome::Applied { .. }),
            "{result:?}"
        );
        session = kernel
            .open(
                SessionSelector::ById {
                    id: session.session.clone(),
                },
                who(),
                OpenOptions::default(),
            )
            .await
            .unwrap();
        assert_eq!(session.snapshot.summary.model.as_deref(), Some(expected));
        if expected == "gpt-5" {
            cycle_efforts(&kernel, &mut session).await;
        }
    }
    assert_eq!(
        std::fs::read(server.cwd().join(".bingo/settings.toml")).unwrap(),
        before
    );
}

async fn cycle_efforts(kernel: &impl HostApi, session: &mut Attachment) {
    let model = kernel
        .catalog(CatalogKind::Models)
        .await
        .unwrap()
        .entries
        .into_iter()
        .find(|m| m.id == "openai/gpt-5")
        .unwrap();
    let efforts: Vec<String> = serde_json::from_value(model.meta["efforts"].clone()).unwrap();
    command(session, "think", json!(efforts.last().unwrap())).await;
    for expected in efforts.iter().chain(efforts.first()) {
        assert!(matches!(
            command(session, "think", json!("next")).await,
            IntentOutcome::Applied { .. }
        ));
        *session = kernel
            .open(
                SessionSelector::ById {
                    id: session.session.clone(),
                },
                who(),
                OpenOptions::default(),
            )
            .await
            .unwrap();
        assert_eq!(session.snapshot.config.kernel["thinking"], *expected);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn disabled_picker_keeps_plain_listing_and_switching() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".bingo")).unwrap();
    std::fs::write(
        home.path().join(".bingo/settings.toml"),
        "[update]\ncheck = false\n[enabledPlugins]\n\"bingo.mod.models\" = false\n",
    )
    .unwrap();
    let mut server = Server::spawn_at(home, SCRIPT, &[]);
    let kernel = ready(&mut server).await;
    let mut session = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    let IntentOutcome::Applied { result } = command(&mut session, "model", json!("")).await else {
        panic!("plain listing")
    };
    let view: View = serde_json::from_value(result["view"].clone()).unwrap();
    assert!(
        view.fold().starts_with("model: fake/fake-1"),
        "{}",
        view.fold()
    );
    assert!(matches!(
        command(&mut session, "model", json!("fake/other")).await,
        IntentOutcome::Applied { .. }
    ));
    session = kernel
        .open(
            SessionSelector::ById {
                id: session.session.clone(),
            },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(session.snapshot.summary.model.as_deref(), Some("other"));
    assert!(!slot::placed(&session.snapshot).iter().any(|placed| {
        matches!(
            &placed.slot,
            slot::Slot::Pane { view: slot::Shown::Pull { request }, .. }
                if request.component == render::Component::ModelPicker
        )
    }));
}

fn picker_open(session: &Attachment) -> bool {
    slot::placed(&session.snapshot).iter().any(|placed| {
        matches!(
            &placed.slot,
            slot::Slot::Pane { view: slot::Shown::Pull { request }, .. }
                if request.component == render::Component::ModelPicker
        )
    })
}
async fn reopen(kernel: &impl HostApi, session: &Attachment) -> Attachment {
    kernel
        .open(
            SessionSelector::ById {
                id: session.session.clone(),
            },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap()
}
#[tokio::test(flavor = "multi_thread")]
async fn unlisted_models_use_explicit_paths_and_sheet_letters_do_nothing() {
    let mut server = Server::spawn(SCRIPT);
    let kernel = ready(&mut server).await;
    let mut session = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    command(&mut session, "model", json!("fake/other")).await;
    session = reopen(&kernel, &session).await;
    assert_eq!(session.snapshot.summary.model.as_deref(), Some("other"));
    assert!(!picker_open(&session));
    command(&mut session, "model", json!("")).await;
    let req = request(&kernel, &mut session).await;
    let id = req.request_id;
    let before = cursor_line(&kernel, &mut session).await;
    for letter in "fakeunlisted".chars() {
        command(
            &mut session,
            "bingo.mod.models:ui",
            press(&id, &letter.to_string()),
        )
        .await;
    }
    assert_eq!(cursor_line(&kernel, &mut session).await, before);
    command(&mut session, "bingo.mod.models:ui", press(&id, "esc")).await;
    command(&mut session, "model", json!("fake/unlisted")).await;
    session = reopen(&kernel, &session).await;
    assert_eq!(session.snapshot.summary.model.as_deref(), Some("unlisted"));
    assert!(!picker_open(&session));
}
#[tokio::test(flavor = "multi_thread")]
async fn keys_act_on_the_kept_row_and_never_on_a_heading() {
    let mut server = Server::spawn(SCRIPT);
    let kernel = ready(&mut server).await;
    let mut session = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    command(&mut session, "model", json!("fake/other")).await;
    command(&mut session, "model", json!("pin fake/fake-1")).await;
    command(&mut session, "model", json!("")).await;
    let req = request(&kernel, &mut session).await;
    let id = req.request_id.clone();
    for key in ["x", "backspace"] {
        command(&mut session, "bingo.mod.models:ui", press(&id, key)).await;
    }
    let req = request(&kernel, &mut session).await;
    let view = kernel.render(&session.session, req).await.unwrap();
    let cursor = picked(&view).unwrap();
    assert!(
        cursor.contains("fake-1") && !cursor.contains("pinned"),
        "{}",
        view.fold()
    );
    // Pins do not duplicate or replace the catalogue row.
    command(&mut session, "model", json!("unpin fake/fake-1")).await;
    command(&mut session, "bingo.mod.models:ui", press(&id, "enter")).await;
    session = reopen(&kernel, &session).await;
    assert_eq!(session.snapshot.summary.model.as_deref(), Some("fake-1"));
    assert!(!picker_open(&session));
}
async fn listing(ids: &[&str]) -> (MockServer, tempfile::TempDir) {
    let data: Vec<_> = ids.iter().map(|id| json!({ "id": id })).collect();
    let endpoint = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": data })))
        .mount(&endpoint)
        .await;
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".bingo")).unwrap();
    std::fs::write(
        home.path().join(".bingo/settings.toml"),
        format!(
            "[update]\ncheck = false\n[openai]\nbaseUrl = \"{}\"\napiKey = \"dummy-local-only\"\n",
            endpoint.uri()
        ),
    )
    .unwrap();
    (endpoint, home)
}
fn picked(view: &View) -> Option<String> {
    if let View::Box { children, layout } = view {
        if layout.ground.as_deref() == Some("pick") {
            return Some(view.fold());
        }
        return children.iter().find_map(picked);
    }
    None
}
async fn cursor_line(kernel: &impl HostApi, session: &mut Attachment) -> String {
    let req = request(kernel, session).await;
    let view = kernel.render(&session.session, req).await.unwrap();
    picked(&view).unwrap_or_else(|| panic!("no cursor: {}", view.fold()))
}
#[tokio::test(flavor = "multi_thread")]
async fn a_queued_enter_takes_the_row_the_cursor_moved_to() {
    let (_endpoint, home) = listing(&["alpha", "beta", "gamma"]).await;
    let mut server = Server::spawn_at(home, SCRIPT, &[]);
    let kernel = ready(&mut server).await;
    let mut session = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    command(&mut session, "models", json!("refresh openai")).await;
    command(&mut session, "model", json!("openai/")).await;
    let id = request(&kernel, &mut session).await.request_id;
    assert!(cursor_line(&kernel, &mut session).await.contains("alpha"));
    command(&mut session, "bingo.mod.models:ui", press(&id, "down")).await;
    // A queued key carries no row identity; the owner keeps the moved selection.
    command(&mut session, "bingo.mod.models:ui", press(&id, "enter")).await;
    session = reopen(&kernel, &session).await;
    assert_eq!(session.snapshot.summary.model.as_deref(), Some("beta"));
}
#[tokio::test(flavor = "multi_thread")]
async fn a_reorder_between_the_move_and_the_enter_keeps_the_row() {
    let (_endpoint, home) = listing(&["alpha", "beta", "gamma"]).await;
    let mut server = Server::spawn_at(home, SCRIPT, &[]);
    let kernel = ready(&mut server).await;
    let mut session = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    command(&mut session, "models", json!("refresh openai")).await;
    command(&mut session, "model", json!("")).await;
    let id = request(&kernel, &mut session).await.request_id;
    for _ in 0..40 {
        if cursor_line(&kernel, &mut session).await.contains("beta") {
            break;
        }
        command(&mut session, "bingo.mod.models:ui", press(&id, "down")).await;
    }
    assert!(cursor_line(&kernel, &mut session).await.contains("beta"));
    // A pin puts a group above every row; beta moves down, the kept row stays beta.
    command(&mut session, "model", json!("pin openai/gamma")).await;
    command(&mut session, "bingo.mod.models:ui", press(&id, "enter")).await;
    session = reopen(&kernel, &session).await;
    assert_eq!(session.snapshot.summary.model.as_deref(), Some("beta"));
}
#[tokio::test(flavor = "multi_thread")]
async fn site_commands_open_their_picker_and_keep_their_argument_hints() {
    let mut server = Server::spawn(SCRIPT);
    let kernel = ready(&mut server).await;
    let specs = kernel
        .catalog(CatalogKind::Commands)
        .await
        .unwrap()
        .entries
        .into_iter()
        .filter_map(|e| serde_json::from_value::<CommandSpec>(e.meta).ok())
        .collect::<Vec<_>>();
    for name in ["model", "think", "models", "login", "logout"] {
        let spec = specs.iter().find(|s| s.name == name).unwrap();
        assert!(!matches!(spec.args, ArgSpec::None), "{name}");
        assert!(spec.opens, "{name}: a palette runs it into the picker");
    }
    let rename = specs.iter().find(|s| s.name == "rename").unwrap();
    assert!(!rename.opens);
}
#[tokio::test(flavor = "multi_thread")]
async fn the_picker_opens_while_an_endpoint_has_not_answered() {
    // Bound and never accepted: the probe's request waits in the backlog for good.
    let silent = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".bingo")).unwrap();
    std::fs::write(
        home.path().join(".bingo/settings.toml"),
        format!(
            r#"[update]
check = false
[openai]
baseUrl = "http://{}"
apiKey = "dummy-local-only"
"#,
            silent.local_addr().unwrap()
        ),
    )
    .unwrap();
    let mut server = Server::spawn_at(home, SCRIPT, &[]);
    let kernel = ready(&mut server).await;
    let mut session = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    assert!(matches!(
        command(&mut session, "model", json!("")).await,
        IntentOutcome::Applied { .. }
    ));
    session = reopen(&kernel, &session).await;
    assert!(picker_open(&session));
    let req = request(&kernel, &mut session).await;
    let id = req.request_id;
    for letter in "gpt".chars() {
        command(
            &mut session,
            "bingo.mod.models:ui",
            press(&id, &letter.to_string()),
        )
        .await;
    }
    let req = request(&kernel, &mut session).await;
    let view = kernel.render(&session.session, req).await.unwrap().fold();
    assert!(view.contains("Model") && view.contains("fake-1"), "{view}");
}
#[tokio::test(flavor = "multi_thread")]
async fn a_switch_from_any_client_is_a_line_in_the_transcript() {
    let mut server = Server::spawn(SCRIPT);
    let kernel = ready(&mut server).await;
    let mut session = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    for surface in ["print", "rpc", "tui"] {
        let intent = IntentId::mint();
        let origin = Origin {
            surface: surface.into(),
            ..Origin::default()
        };
        session
            .handle
            .submit(intent.clone(), Input::text("/model fake/other", origin));
        let IntentOutcome::Applied { result } = ack_for(&mut session, &intent).await else {
            panic!("{surface}: the switch is applied");
        };
        let message = result["message"].as_str().unwrap_or_default();
        assert!(message.starts_with("⇄ other · fake"), "{surface}: {result}");
        let item = result["item"].as_str().expect("the receipt is recorded");
        session = reopen(&kernel, &session).await;
        let recorded = session
            .snapshot
            .transcript
            .items()
            .iter()
            .find(|i| i.id.to_string() == item)
            .cloned()
            .expect("the receipt is in the transcript");
        assert_eq!(recorded.intent.as_ref(), Some(&intent));
        assert!(matches!(
            recorded.body,
            ItemBody::Notice { ref text, .. } if text == message
        ));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn typed_surface_opens_the_picker_independently_of_attribution() {
    let mut server = Server::spawn(SCRIPT);
    let kernel = ready(&mut server).await;
    let mut session = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    for surface in [
        None,
        Some(render::SurfaceName::Text),
        Some(render::SurfaceName::Terminal),
    ] {
        let intent = IntentId::mint();
        let origin = Origin {
            render_surface: surface,
            ..Origin::surface("tui")
        };
        session
            .handle
            .submit(intent.clone(), Input::text("/model", origin));
        let outcome = ack_for(&mut session, &intent).await;
        assert!(
            matches!(outcome, IntentOutcome::Applied { .. }),
            "{outcome:?}"
        );
        session = reopen(&kernel, &session).await;
        let picker = slot::placed(&session.snapshot).into_iter().any(|placed| {
            matches!(
                placed.slot, slot::Slot::Pane { view: slot::Shown::Pull { request }, .. }
                if request.component == render::Component::ModelPicker
            )
        });
        assert_eq!(picker, surface == Some(render::SurfaceName::Terminal));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_sheet_choice_records_no_notice_and_publishes_one_toast() {
    let mut server = Server::spawn(SCRIPT);
    let kernel = ready(&mut server).await;
    let mut session = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    command(&mut session, "model", json!("")).await;
    let id = request(&kernel, &mut session).await.request_id;
    let intent = IntentId::mint();
    session.handle.submit(
        intent.clone(),
        Input::action(
            Action {
                name: "bingo.mod.models:ui".into(),
                args: press(&id, "enter"),
            },
            Origin::surface("rpc").on(render::SurfaceName::Terminal),
        ),
    );
    let (mut toasts, mut notices) = (0, 0);
    tokio::time::timeout(LIMIT, async {
        while let Some(frame) = session.events.next().await {
            match frame.event {
                Event::Signal { payload, .. } => {
                    toasts += payload["slots"].as_array().map_or(0, |slots| {
                        slots
                            .iter()
                            .filter(|slot| slot["kind"] == "toast" || slot["kind"] == "Toast")
                            .count()
                    });
                }
                Event::ItemCompleted { item } if matches!(item.body, ItemBody::Notice { .. }) => {
                    notices += 1
                }
                Event::IntentAck {
                    intent: ack,
                    outcome,
                } if ack == intent => {
                    assert!(matches!(outcome, IntentOutcome::Applied { .. }));
                    return;
                }
                _ => {}
            }
        }
        panic!("sheet choice did not acknowledge");
    })
    .await
    .unwrap();
    assert_eq!(notices, 0);
    assert_eq!(toasts, 1);
}
