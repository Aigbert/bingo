use super::*;
#[tokio::test(flavor = "multi_thread")]
async fn fork_two_turns_lists_a_root_with_its_origin_and_kept_ids() {
    let mut server =
        Server::spawn(r#"{"responses":[{"steps":[{"text":"one"}]},{"steps":[{"text":"two"}]}]}"#);
    let kernel = ready(&mut server).await;
    let mut source = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    source.handle.submit(
        IntentId::mint(),
        Input::text("first ask", Origin::surface("test")),
    );
    until_completed(&mut source).await;
    let first = source.snapshot.clone();
    source.handle.submit(
        IntentId::mint(),
        Input::text("second ask", Origin::surface("test")),
    );
    until_completed(&mut source).await;
    let turn = first.last_turn.as_ref().unwrap().id.clone();
    let id = kernel
        .fork(&source.session, Some(turn.clone()))
        .await
        .unwrap();
    let list = kernel.sessions(SessionFilter::default()).await.unwrap();
    let summary = list.iter().find(|s| s.id == id).unwrap();
    assert!(summary.parent.is_none());
    assert_eq!(
        summary.history.forked_from,
        Some(bingo_sdk::ForkLink {
            session: source.session.clone(),
            turn: Some(turn)
        })
    );
    let fork = kernel
        .open(SessionSelector::ById { id }, who(), OpenOptions::default())
        .await
        .unwrap();
    assert_eq!(fork.snapshot.transcript.items(), first.transcript.items());
    kernel.shutdown().await.unwrap();
    assert!(server.child.wait().await.unwrap().success());
}
#[tokio::test(flavor = "multi_thread")]
async fn fork_on_the_wire_refuses_a_busy_source() {
    let mut server =
        Server::spawn(r#"{"responses":[{"steps":[{"text":"waiting"},{"delay":{"ms":30000}}]}]}"#);
    let kernel = ready(&mut server).await;
    let mut source = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
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
        kernel.fork(&source.session, None).await.unwrap_err().code,
        ErrorCode::NotReady
    );
    kernel.shutdown().await.unwrap();
    assert!(server.child.wait().await.unwrap().success());
}

#[tokio::test]
async fn fork_stdout_is_only_ndjson_including_notifications_and_shutdown() {
    let mut server = Server::spawn(TEXT_TURN);
    let mut stdin = server.child.take_stdin().unwrap();
    let mut lines = BufReader::new(server.child.take_stdout().unwrap()).lines();
    let mut reply = async |id: u32| -> serde_json::Value {
        loop {
            let line = tokio::time::timeout(LIMIT, lines.next_line())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            let value: serde_json::Value = serde_json::from_str(&line)
                .unwrap_or_else(|error| panic!("non-JSON stdout: {error}: {line}"));
            assert_eq!(value["jsonrpc"], "2.0", "{line}");
            if value["id"] == id {
                return value;
            }
        }
    };
    send(&mut stdin, r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"client":{"name":"raw","surface":"test"},"protocol":1}}"#).await;
    reply(1).await;
    let open = serde_json::json!({"jsonrpc":"2.0","id":2,"method":"session/open","params":{"selector":create(server.cwd()),"client":who()}});
    send(&mut stdin, &open.to_string()).await;
    let opened = reply(2).await;
    let source = &opened["result"]["session"];
    assert!(source.is_string(), "{opened}");
    send(&mut stdin, &serde_json::json!({"jsonrpc":"2.0","id":3,"method":"session/fork","params":{"session":source}}).to_string()).await;
    let fork = reply(3).await;
    assert!(fork.get("error").is_none(), "{fork}");
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":4,"method":"shutdown","params":{}}"#,
    )
    .await;
    reply(4).await;
    while let Some(line) = tokio::time::timeout(LIMIT, lines.next_line())
        .await
        .unwrap()
        .unwrap()
    {
        let value: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["jsonrpc"], "2.0");
    }
    assert!(server.child.wait().await.unwrap().success());
}
