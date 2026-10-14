use crate::*;
use serde_json::{Value, json};

struct Recording {
    inner: SessionHandle,
    inputs: Arc<Mutex<Vec<Input>>>,
}
#[async_trait::async_trait]
impl SessionPort for Recording {
    fn submit(&self, _: IntentId, input: Input) {
        self.inputs.lock().unwrap().push(input);
    }
    fn interrupt(&self, id: IntentId, scope: InterruptScope) {
        self.inner.interrupt(id, scope);
    }
    fn answer(
        &self,
        id: IntentId,
        interaction: InteractionId,
        answer: Answer,
        activation: Activation,
    ) {
        self.inner.answer(id, interaction, answer, activation);
    }
    async fn history(&self, page: HistoryPage) -> Result<HistoryChunk, KernelError> {
        self.inner.history(page).await
    }
    async fn events_since(&self, since: Seq) -> Result<FrameStream, KernelError> {
        self.inner.events_since(since).await
    }
}
fn host(state: SessionState, inputs: Arc<Mutex<Vec<Input>>>) -> testing::TestHost {
    let table = testing::TestHost::with_sessions(vec![state.summary.clone()]).handle();
    let host = testing::TestHost::default();
    host.on_open(move |selector, who, options| {
        let (table, state, inputs) = (table.clone(), state.clone(), inputs.clone());
        async move {
            let mut attachment = table.open(selector, who, options).await?;
            attachment.snapshot = state;
            attachment.handle = SessionHandle(Arc::new(Recording {
                inner: attachment.handle,
                inputs,
            }));
            Ok(attachment)
        }
    });
    host.on_catalog(|kind| async move {
        Ok(Catalog {
            kind,
            entries: vec![],
        })
    });
    host.on_signal(|_, _, _, _| async { Ok(()) });
    host
}
#[tokio::test]
async fn enter_submits_each_rows_command_after_removing_the_card() {
    let Event::SessionUpdated { summary } = serde_json::from_str(include_str!(
        "../../bingo-sdk/fixtures/record/event/sessionUpdated.json"
    ))
    .unwrap() else {
        panic!("summary")
    };
    let mut state = SessionState::new(summary);
    state.config.plugins.insert(
        "bingo.permissions".into(),
        json!({"rules":["Edit(src/**)", "Bash(cargo:*)"]}),
    );
    let inputs = Arc::new(Mutex::new(Vec::new()));
    let host = host(state.clone(), inputs.clone());
    let cx = CommandContext {
        session: state.summary.id.clone(),
        cwd: "/unused".into(),
        host: host.handle(),
        cancel: Default::default(),
    };
    let selections = Selections::default();
    let handler = keys::Handler(selections.clone());
    for row in rows::rows(&(&state).into(), &[], &[], None)
        .into_iter()
        .filter(|r| {
            r.command
                .as_deref()
                .is_some_and(|command| !matches!(command, "rename" | "permission"))
        })
    {
        selections
            .lock()
            .unwrap()
            .insert(cx.session.clone(), row.label.into());
        let outcome = handler
            .run(
                &json!({"handler":"status","event":"press","value":{"key":"enter"},"rows":rows::rows(&(&state).into(), &[], &[], None)}).to_string(),
                &cx,
            )
            .await
            .unwrap_err();
        // The recording port has no acknowledgement stream; it must not claim success.
        assert_eq!(outcome.code, ErrorCode::SessionClosed);
        assert!(!selections.lock().unwrap().contains_key(&cx.session));
        let command = row.command.unwrap();
        let (name, args) = command.split_once(' ').unwrap_or((&command, ""));
        assert_eq!(
            inputs.lock().unwrap().pop(),
            Some(Input::action(
                Action {
                    name: name.into(),
                    args: Value::String(args.into())
                },
                Origin::surface("plugin:bingo.mod.status")
            ))
        );
        assert!(host.calls().iter().any(|call| matches!(
            call,
            testing::HostCall::Signal {
                payload: Value::Null,
                ..
            }
        )));
    }
}

#[tokio::test]
async fn editable_rows_prefill_instead_of_submitting_bare_commands() {
    let Event::SessionUpdated { summary } = serde_json::from_str(include_str!(
        "../../bingo-sdk/fixtures/record/event/sessionUpdated.json"
    ))
    .unwrap() else {
        panic!("summary")
    };
    let state = SessionState::new(summary);
    let inputs = Arc::new(Mutex::new(Vec::new()));
    let host = host(state.clone(), inputs.clone());
    let cx = CommandContext {
        session: state.summary.id.clone(),
        cwd: "/unused".into(),
        host: host.handle(),
        cancel: Default::default(),
    };
    let selections = Selections::default();
    for (label, text) in [("session", "/rename "), ("permission", "/permission ")] {
        selections
            .lock()
            .unwrap()
            .insert(cx.session.clone(), label.into());
        let outcome = keys::Handler(selections.clone()).run(&json!({"handler":"status","event":"press","value":{"key":"enter"},"rows":rows::rows(&(&state).into(), &[], &[], None)}).to_string(), &cx).await.unwrap();
        assert_eq!(
            outcome,
            CommandOutcome::Surface {
                action: Action {
                    name: "Composer".into(),
                    args: json!({"text":text})
                },
                fallback: Some(text.into())
            }
        );
        assert!(inputs.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn a_visible_key_resumes_an_expired_card_without_a_session_read() {
    let state = crate::tests::state();
    let host = testing::TestHost::default();
    let cx = CommandContext {
        session: state.summary.id.clone(),
        cwd: "/unused".into(),
        host: host.handle(),
        cancel: Default::default(),
    };
    let selections = Selections::default();
    let outcome = keys::Handler(selections.clone()).run(&json!({"handler":"status","event":"press","value":{"key":"down"},"selected":"model","rows":rows::rows(&(&state).into(), &[], &[], None)}).to_string(), &cx).await.unwrap();
    assert_eq!(outcome, CommandOutcome::Handled { handled: true });
    assert_eq!(selections.lock().unwrap()[&cx.session].label, "context");
    assert!(matches!(
        host.calls().as_slice(),
        [testing::HostCall::Invalidate { .. }]
    ));
}
