use super::*;
#[tokio::test]
async fn fork_keeps_ids_configuration_and_the_cut_but_no_children() {
    let store = Arc::new(crate::journal::MemoryStore::new());
    let provider = ScriptedProvider::new(vec![
        Script::Events(text("one")),
        Script::Events(text("two")),
    ]);
    let host = host_on(store.clone(), provider).await;
    let mut source = host
        .open(
            SessionSelector::Create {
                spec: spec("/work"),
            },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    one_turn(&mut source, "first").await;
    let first = host.session_state(&source.session).await.unwrap();
    one_turn(&mut source, "second").await;
    let child = host
        .open(
            SessionSelector::Create {
                spec: SessionSpec {
                    cwd: "/work".into(),
                    driver: Driver::Log,
                    parent: Some(ParentLink {
                        session: source.session.clone(),
                        item: None,
                    }),
                    ..Default::default()
                },
            },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(
        child.snapshot.summary.parent.as_ref().unwrap().session,
        source.session
    );
    let cut = first.last_turn.as_ref().unwrap().id.clone();
    let id = host.fork(&source.session, Some(cut.clone())).await.unwrap();
    let fork = host
        .open(
            SessionSelector::ById { id: id.clone() },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(fork.snapshot.transcript.items(), first.transcript.items());
    assert_eq!(
        fork.snapshot.summary.history.first_ask.as_deref(),
        Some("first")
    );
    assert_eq!(fork.snapshot.summary.model, first.summary.model);
    assert!(
        host.sessions(SessionFilter {
            parent: Some(id.clone()),
            ..Default::default()
        })
        .await
        .unwrap()
        .is_empty()
    );
    assert_eq!(
        host.sessions(SessionFilter {
            parent: Some(source.session.clone()),
            ..Default::default()
        })
        .await
        .unwrap()
        .len(),
        1
    );
    assert!(fork.snapshot.summary.parent.is_none());
    assert!(fork.snapshot.summary.key.is_none());
    assert_eq!(
        fork.snapshot.summary.history.forked_from,
        Some(ForkLink {
            session: source.session.clone(),
            turn: Some(cut)
        })
    );
    assert!(
        store
            .replay(&id, Seq::ZERO)
            .await
            .unwrap()
            .iter()
            .all(|f| f.session == id)
    );
    assert_eq!(
        host.fork(&source.session, Some(TurnId::from_raw("unknown")))
            .await
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    let whole = host.fork(&source.session, None).await.unwrap();
    assert_ne!(whole, id);
    host.shutdown().await;
}
#[tokio::test]
async fn fork_refuses_a_running_turn() {
    let store = Arc::new(crate::journal::MemoryStore::new());
    let provider = ScriptedProvider::new(vec![Script::Hang(vec![])]);
    let host = host_on(store, provider).await;
    let mut source = host
        .open(
            SessionSelector::Create {
                spec: spec("/work"),
            },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    source.handle.submit(
        IntentId::mint(),
        Input::text("wait", Origin::surface("test")),
    );
    while let Some(frame) = source.events.next().await {
        if matches!(frame.event, Event::TurnStarted { .. }) {
            break;
        }
    }
    assert_eq!(
        host.fork(&source.session, None).await.unwrap_err().code,
        ErrorCode::NotReady
    );
    assert_eq!(
        host.delete(&source.session).await.unwrap_err().code,
        ErrorCode::NotReady
    );
    assert!(
        host.read(&source.session).await.unwrap().busy(),
        "delete must not end the turn"
    );
    host.shutdown().await;
}
