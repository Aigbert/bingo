use super::*;

async fn cancel_readdress(session: &mut Attachment, url: &str) {
    let intent = IntentId::mint();
    session.handle.submit(
        intent.clone(),
        Input::text(
            format!("/provider set work url {url}"),
            Origin::surface("test"),
        ),
    );
    let dialog = tokio::time::timeout(LIMIT, async {
        loop {
            let frame = session.events.next().await.unwrap();
            session.snapshot.apply(&frame);
            if let Event::InteractionOpened { interaction } = frame.event {
                break interaction;
            }
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        dialog.kind,
        InteractionKind::Login {
            flow: LoginFlow::Paste,
            ..
        }
    ));
    session.handle.answer(
        IntentId::mint(),
        dialog.id,
        Answer::Cancel,
        Activation::Programmatic,
    );
    let _ = ack_for(session, &intent).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn returning_to_an_address_cannot_revive_the_removed_settings_key() {
    for family in ["openai", "anthropic"] {
        let first = endpoint("First").await;
        let second = endpoint("Second").await;
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join(".bingo")).unwrap();
        let file = home.path().join(".bingo/settings.toml");
        std::fs::write(&file, format!(
            "[update]\ncheck = false\n[{family}.instances.work]\nbaseUrl = {:?}\napiKey = \"dummy-configured\"\n", first.uri()
        )).unwrap();
        let mut server = Server::spawn_at(home, IDLE, &[]);
        let kernel = ready(&mut server).await;
        let mut session = kernel
            .open(create(server.cwd()), who(), OpenOptions::default())
            .await
            .unwrap();
        assert_eq!(meta(&kernel).await["auth"]["kind"], "ready");
        cancel_readdress(&mut session, &second.uri()).await;
        let saved = bingo_core::settings::read_document(&file).unwrap();
        assert!(saved[family]["instances"]["work"].get("apiKey").is_none());
        cancel_readdress(&mut session, &first.uri()).await;
        assert_eq!(meta(&kernel).await["auth"]["kind"], "missing");
        first.reset().await;
        let _ = command(&mut session, "/models refresh work").await;
        assert!(first.received_requests().await.unwrap().is_empty());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn same_origin_edit_rereads_a_changed_settings_key() {
    let endpoint = endpoint("First").await;
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".bingo")).unwrap();
    let file = home.path().join(".bingo/settings.toml");
    let settings = |key| {
        format!(
            "[update]\ncheck = false\n[openai.instances.work]\nbaseUrl = {:?}\napiKey = {key:?}\n",
            endpoint.uri()
        )
    };
    std::fs::write(&file, settings("old-dummy")).unwrap();
    let mut server = Server::spawn_at(home, IDLE, &[]);
    let kernel = ready(&mut server).await;
    let mut session = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    std::fs::write(&file, settings("fresh-dummy")).unwrap();
    endpoint.reset().await;
    applied(
        &mut session,
        &format!("/provider set work url {}/new", endpoint.uri()),
    )
    .await;
    let requests = endpoint.received_requests().await.unwrap();
    assert!(!requests.is_empty());
    assert!(
        requests
            .iter()
            .all(|request| request.headers.get("authorization").unwrap() == "Bearer fresh-dummy")
    );
}
