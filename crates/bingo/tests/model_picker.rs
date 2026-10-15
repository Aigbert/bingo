//! Model-site actions use only local providers and private credential answers.
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
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
    session.handle.submit(
        intent.clone(),
        Input::Action {
            origin: Some(Origin::surface("rpc")),
            action: Action {
                name: name.into(),
                args,
            },
        },
    );
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
            origin: Some(Origin::surface("rpc")),
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
async fn pasted_key_is_masked_in_frames_and_absent_from_the_journal() {
    let endpoint = endpoint("picker-local").await;
    let mut server = Server::spawn(SCRIPT);
    let kernel = ready(&mut server).await;
    let mut session = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    assert!(matches!(
        command(&mut session, "model", json!("")).await,
        IntentOutcome::Applied { .. }
    ));
    let req = request(&kernel, &mut session).await;
    let id = req.request_id;
    private_press(&mut session, &id, "+", SECRET).await;
    let req = request(&kernel, &mut session).await;
    let view = kernel.render(&session.session, req).await.unwrap();
    assert!(view.fold().contains("••••••••"), "{}", view.fold());
    assert!(!serde_json::to_string(&view).unwrap().contains(SECRET));
    // The guessed vendor URL is replaced before any command can send the dummy key.
    command(&mut session, "bingo.mod.models:ui", press(&id, "down")).await;
    command(&mut session, "bingo.mod.models:ui", press(&id, "down")).await;
    private_press(
        &mut session,
        &id,
        "enter",
        &format!("{}/v1", endpoint.uri()),
    )
    .await;
    command(&mut session, "bingo.mod.models:ui", press(&id, "down")).await;
    command(&mut session, "bingo.mod.models:ui", press(&id, "down")).await;
    let outcome = command(&mut session, "bingo.mod.models:ui", press(&id, "enter")).await;
    assert!(matches!(outcome, IntentOutcome::Applied { .. }));
    let models = kernel.catalog(CatalogKind::Models).await.unwrap();
    assert!(
        models
            .entries
            .iter()
            .any(|m| m.id == "openai-2/picker-local")
    );
    assert!(!endpoint.received_requests().await.unwrap().is_empty());
    command(&mut session, "bingo.mod.models:ui", press(&id, "esc")).await;
    command(&mut session, "bingo.mod.models:ui", press(&id, "enter")).await;
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
    assert_eq!(
        attached.snapshot.summary.model.as_deref(),
        Some("picker-local")
    );
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
    assert!(view.fold().contains("fake/fake-1"));
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
async fn unlisted_models_switch_by_path_and_by_the_typed_filter() {
    let mut server = Server::spawn(SCRIPT);
    let kernel = ready(&mut server).await;
    let mut session = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    assert!(matches!(
        command(&mut session, "model", json!("fake/other")).await,
        IntentOutcome::Applied { .. }
    ));
    session = reopen(&kernel, &session).await;
    assert_eq!(session.snapshot.summary.model.as_deref(), Some("other"));
    assert!(!picker_open(&session));
    command(&mut session, "model", json!("")).await;
    let req = request(&kernel, &mut session).await;
    let id = req.request_id;
    for letter in "fake/unlisted".chars() {
        command(
            &mut session,
            "bingo.mod.models:ui",
            press(&id, &letter.to_string()),
        )
        .await;
    }
    let req = request(&kernel, &mut session).await;
    let view = kernel.render(&session.session, req).await.unwrap().fold();
    assert!(view.contains("⏎ use fake/unlisted"), "{view}");
    command(&mut session, "bingo.mod.models:ui", press(&id, "enter")).await;
    session = reopen(&kernel, &session).await;
    assert_eq!(session.snapshot.summary.model.as_deref(), Some("unlisted"));
    assert!(!picker_open(&session));
}
