use super::*;

async fn next_prompt(session: &mut Attachment) -> Interaction {
    tokio::time::timeout(LIMIT, async {
        loop {
            let frame = session.events.next().await.unwrap();
            session.snapshot.apply(&frame);
            if let Event::InteractionOpened { interaction } = frame.event {
                return interaction;
            }
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn repository_sign_in_names_destination_before_any_credential() {
    for family in ["openai", "anthropic", "codex"] {
        let endpoint = endpoint("unused").await;
        let home = tempfile::tempdir().unwrap();
        let root = home.path().join("project");
        std::fs::create_dir_all(root.join(".bingo")).unwrap();
        let file = root.join(".bingo/settings.toml");
        let field = if family == "codex" {
            "issuer"
        } else {
            "baseUrl"
        };
        std::fs::write(
            &file,
            format!(
                "[{family}.instances.work]\n{field} = \"{}\"\n",
                endpoint.uri()
            ),
        )
        .unwrap();
        let auth = home.path().join(".bingo/data/auth.json");
        let mut server = Server::spawn_in(home, IDLE, &[], &root);
        let kernel = ready(&mut server).await;
        let mut session = kernel
            .open(create(root), who(), OpenOptions::default())
            .await
            .unwrap();
        let intent = IntentId::mint();
        session.handle.submit(
            intent.clone(),
            Input::text("/login work paste", Origin::surface("rpc")),
        );
        let dialog = next_prompt(&mut session).await;
        let InteractionKind::Confirm { detail, .. } = dialog.kind else {
            panic!("destination missing: {:?}", dialog.kind)
        };
        assert!(detail.contains(&endpoint.uri()), "{detail}");
        assert!(detail.contains(&file.display().to_string()), "{detail}");
        assert!(endpoint.received_requests().await.unwrap().is_empty());
        session.handle.answer(
            IntentId::mint(),
            dialog.id,
            Answer::Cancel,
            Activation::Programmatic,
        );
        let _ = ack_for(&mut session, &intent).await;
        assert!(!auth.exists());
        assert!(endpoint.received_requests().await.unwrap().is_empty());
        kernel.shutdown().await.unwrap();
        assert!(
            tokio::time::timeout(LIMIT, server.child.wait())
                .await
                .unwrap()
                .unwrap()
                .success()
        );
    }
}
