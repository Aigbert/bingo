//! The ask door (ADR-0039): who answers a question no tool defines.
//!
//! Nothing here waits on a clock. An interaction has no timeout — a gate
//! question with nobody attached waits exactly as long as its session lives —
//! so "unanswerable" is the policy's own refusing stance, and the questions
//! that end without a person end because a person interrupted or the session
//! closed under them.

use super::*;

static STANCE: PluginManifest = PluginManifest {
    id: "test.stance",
    version: "0",
    uses: &[],
    provides: &["policy:stance"],
    requires: &[],
    config: None,
};

/// Stands where it was told to, and asks about every call.
struct StancePolicy(Stance);

#[async_trait]
impl PermissionPolicy for StancePolicy {
    fn id(&self) -> &str {
        "stance"
    }
    async fn decide(&self, _: PolicyInput<'_>) -> Decision {
        Decision::Ask {
            reason: Reason::Default,
            scope: None,
        }
    }
    async fn stance(&self, _: &SessionId) -> Stance {
        self.0
    }
}

async fn standing(stance: Stance, scripts: Vec<Script>) -> Arc<Host> {
    let plugins = vec![
        TestPlugin::boxed(
            &PROVIDER,
            vec![Contribution::Provider(ScriptedProvider::new(scripts))],
        ),
        TestPlugin::boxed(
            &STANCE,
            vec![Contribution::Policy(Arc::new(StancePolicy(stance)))],
        ),
    ];
    let config = HostConfig::new(env()).with_layer("cli", json!({ "model": "m" }));
    Host::build(plugins, config).await.unwrap()
}

async fn attach(host: &Host) -> Attachment {
    host.open(
        SessionSelector::Create {
            spec: spec("/work"),
        },
        who(),
        OpenOptions::default(),
    )
    .await
    .unwrap()
}

fn option(id: &str, role: Option<AnswerRole>) -> QuestionOption {
    QuestionOption {
        id: id.into(),
        label: format!("the agent's word for {id}"),
        description: None,
        role,
        preview: None,
    }
}

/// An asker's own options, two of them marked: this is what a
/// `session/request_permission` becomes (ADR-0039 §3).
fn question(options: Vec<QuestionOption>) -> InteractionKind {
    InteractionKind::Question(Question {
        question: "may I write to /etc/hosts?".into(),
        header: Some("Permission".into()),
        options,
        free_text: false,
        multi: false,
    })
}

fn marked() -> InteractionKind {
    question(vec![
        option("allow_once", Some(AnswerRole::Allowing)),
        option("allow_always", None),
        option("reject_once", Some(AnswerRole::Refusing)),
    ])
}

fn answers() -> Vec<AnswerSpec> {
    vec![AnswerSpec::Choice, AnswerSpec::Cancel]
}

fn picked(id: &str) -> Answer {
    Answer::Choice {
        ids: vec![id.into()],
        other: None,
    }
}

/// Every frame the session published up to a mark the door cannot have
/// raced: the actor reads its mailbox in order, so anything the ask opened is
/// already here by the time the mark arrives.
async fn frames_to_the_mark(host: &Host, attachment: &mut Attachment) -> Vec<Event> {
    host.extend(&attachment.session, "test", "mark", json!(1))
        .await
        .unwrap();
    let mut seen = Vec::new();
    while let Some(frame) = attachment.events.next().await {
        let mark = matches!(&frame.event, Event::Extension { kind, .. } if kind == "mark");
        seen.push(frame.event);
        if mark {
            break;
        }
    }
    seen
}

/// The interaction the next question opens.
async fn opened(attachment: &mut Attachment) -> InteractionId {
    while let Some(frame) = attachment.events.next().await {
        if let Event::InteractionOpened { interaction } = frame.event {
            return interaction.id;
        }
    }
    panic!("no question was ever opened");
}

#[tokio::test]
async fn an_allowing_stance_answers_the_question_and_opens_nothing() {
    let host = standing(Stance::Allow, vec![]).await;
    let mut attachment = attach(&host).await;
    let answer = host
        .ask(&attachment.session.clone(), marked(), answers())
        .await
        .unwrap();
    assert_eq!(answer, picked("allow_once"));
    let frames = frames_to_the_mark(&host, &mut attachment).await;
    assert!(
        !frames
            .iter()
            .any(|e| matches!(e, Event::InteractionOpened { .. })),
        "nobody was asked, so no interaction was opened: {frames:?}"
    );
    assert!(
        !frames
            .iter()
            .any(|e| matches!(e, Event::ItemStarted { .. })),
        "and nothing was journaled for it: {frames:?}"
    );
}

#[tokio::test]
async fn a_refusing_stance_answers_with_the_fail_closed_option_at_once() {
    let host = standing(Stance::Refuse, vec![]).await;
    let mut attachment = attach(&host).await;
    let answer = host
        .ask(&attachment.session.clone(), marked(), answers())
        .await
        .unwrap();
    assert_eq!(answer, picked("reject_once"));
    let frames = frames_to_the_mark(&host, &mut attachment).await;
    assert!(
        !frames
            .iter()
            .any(|e| matches!(e, Event::InteractionOpened { .. })),
        "a session with nobody at it asks nobody: {frames:?}"
    );
}

#[tokio::test]
async fn a_question_between_turns_reaches_the_person_and_their_answer_comes_back() {
    let host = standing(Stance::Ask, vec![]).await;
    let mut attachment = attach(&host).await;
    let session = attachment.session.clone();
    let asking = tokio::spawn({
        let host = Arc::clone(&host);
        async move { host.ask(&session, marked(), answers()).await }
    });

    let id = opened(&mut attachment).await;
    attachment.handle.answer(
        IntentId::mint(),
        id,
        picked("allow_always"),
        Activation::Pointer,
    );
    assert_eq!(
        asking.await.unwrap().unwrap(),
        picked("allow_always"),
        "the person's own choice, whichever option it was"
    );
}

#[tokio::test]
async fn a_question_under_a_running_turn_is_refused_when_the_turn_is_interrupted() {
    let host = standing(Stance::Ask, vec![Script::Hang(vec![])]).await;
    let mut attachment = attach(&host).await;
    let session = attachment.session.clone();
    attachment
        .handle
        .submit(IntentId::mint(), Input::text("hi", Origin::surface("test")));
    while let Some(frame) = attachment.events.next().await {
        if matches!(frame.event, Event::TurnStarted { .. }) {
            break;
        }
    }

    let asking = tokio::spawn({
        let host = Arc::clone(&host);
        async move { host.ask(&session, marked(), answers()).await }
    });
    opened(&mut attachment).await;
    attachment
        .handle
        .interrupt(IntentId::mint(), InterruptScope::Head);
    assert_eq!(
        asking.await.unwrap().unwrap(),
        picked("reject_once"),
        "the asker hears the refusal, not an error it would have to read as one"
    );
}

#[tokio::test]
async fn a_question_the_session_closes_under_is_refused_rather_than_left_waiting() {
    let host = standing(Stance::Ask, vec![]).await;
    let mut attachment = attach(&host).await;
    let session = attachment.session.clone();
    let asking = tokio::spawn({
        let host = Arc::clone(&host);
        let session = session.clone();
        async move { host.ask(&session, marked(), answers()).await }
    });
    opened(&mut attachment).await;
    host.delete(&session).await.unwrap();
    assert_eq!(asking.await.unwrap().unwrap(), picked("reject_once"));
}

#[tokio::test]
async fn a_question_that_marks_no_option_is_never_answered_for_the_person() {
    let host = standing(Stance::Allow, vec![]).await;
    let attachment = attach(&host).await;
    let unmarked = question(vec![option("allow_once", None)]);
    let error = host
        .ask(&attachment.session, unmarked, answers())
        .await
        .expect_err("a question with nothing to answer it by");
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert!(error.message.contains("allowing"), "{}", error.message);
}

/// A sign-in names no option a stance could pick, and is not the policy's
/// to weigh: `/mcp login` in a bypass session asks the person as `/login`
/// does, rather than being refused for naming no allowing option.
#[tokio::test]
async fn a_sign_in_is_put_to_the_person_whatever_the_stance() {
    let host = standing(Stance::Allow, vec![]).await;
    let mut attachment = attach(&host).await;
    let session = attachment.session.clone();
    let asking = tokio::spawn({
        let host = Arc::clone(&host);
        async move {
            host.ask(
                &session,
                InteractionKind::Login {
                    provider: "binlesson".into(),
                    flow: LoginFlow::Browser {
                        url: "https://as.example.com/authorize".into(),
                    },
                },
                vec![AnswerSpec::Cancel],
            )
            .await
        }
    });
    let id = opened(&mut attachment).await;
    attachment
        .handle
        .answer(IntentId::mint(), id, Answer::Cancel, Activation::Pointer);
    assert_eq!(asking.await.unwrap().unwrap(), Answer::Cancel);
}

#[tokio::test]
async fn a_session_this_host_does_not_run_is_never_answered_for() {
    let host = standing(Stance::Allow, vec![]).await;
    let error = host
        .ask(&SessionId::from_raw("ses_nowhere"), marked(), answers())
        .await
        .expect_err("no such session");
    assert_eq!(error.code, ErrorCode::SessionNotFound);
}

struct BorrowedHost(Arc<std::sync::Mutex<Option<HostHandle>>>);
#[async_trait]
impl Plugin for BorrowedHost {
    fn manifest(&self) -> &'static PluginManifest {
        &STANCE
    }
    fn register(&self, _: &mut Registrar) -> Result<(), PluginError> {
        Ok(())
    }
    async fn start(&self, host: HostHandle) -> Result<(), PluginError> {
        *self.0.lock().unwrap() = Some(host);
        Ok(())
    }
}

async fn plugin_host() -> (Arc<Host>, HostHandle) {
    let borrowed = Arc::new(std::sync::Mutex::new(None));
    let plugins: Vec<Box<dyn Plugin>> = vec![
        TestPlugin::boxed(
            &PROVIDER,
            vec![Contribution::Provider(ScriptedProvider::new(vec![]))],
        ),
        Box::new(BorrowedHost(borrowed.clone())),
    ];
    let host = Host::build(
        plugins,
        HostConfig::new(env()).with_layer("cli", json!({"model":"m"})),
    )
    .await
    .unwrap();
    let plugin = borrowed.lock().unwrap().clone().unwrap();
    (host, plugin)
}

#[tokio::test]
async fn a_plugins_host_handle_cannot_approve_a_permission() {
    let (host, plugin) = plugin_host().await;
    let mut person = attach(&host).await;
    let session = person.session.clone();
    let asking = host.clone();
    let pending = tokio::spawn(async move {
        asking
            .ask(
                &session,
                serde_json::from_value(
                    json!({"kind":"permission","tool":"Echo","summary":"may I?"}),
                )
                .unwrap(),
                vec![AnswerSpec::AllowOnce, AnswerSpec::Deny],
            )
            .await
    });
    let id = opened(&mut person).await;
    let forged = forged(&plugin, &person.session).await;
    for activation in [
        Activation::Programmatic,
        Activation::Keyboard,
        Activation::Pointer,
    ] {
        let intent = IntentId::mint();
        forged
            .handle
            .answer(intent.clone(), id.clone(), Answer::AllowOnce, activation);
        let events = frames_to_the_mark(&host, &mut person).await;
        assert!(events.iter().any(|e| matches!(e, Event::IntentAck { intent: i, outcome: IntentOutcome::Rejected { error } } if i == &intent && error.code == ErrorCode::PermissionDenied)), "{events:?}");
    }
    person.handle.answer(
        IntentId::mint(),
        id,
        Answer::Deny { feedback: None },
        Activation::Programmatic,
    );
    assert!(matches!(
        pending.await.unwrap().unwrap(),
        Answer::Deny { .. }
    ));
    host.shutdown().await;
}

async fn forged(plugin: &HostHandle, session: &SessionId) -> Attachment {
    plugin
        .open(
            SessionSelector::ById {
                id: session.clone(),
            },
            ClientIdentity {
                name: "ask".into(),
                surface: "terminal".into(),
            },
            OpenOptions {
                children: true,
                ..Default::default()
            },
        )
        .await
        .unwrap()
}
#[tokio::test]
async fn accepted_plugin_answers_record_their_source() {
    let (host, plugin) = plugin_host().await;
    let mut person = attach(&host).await;
    let asking = host.clone();
    let session = person.session.clone();
    let pending = tokio::spawn(async move { asking.ask(&session, marked(), answers()).await });
    let id = opened(&mut person).await;
    let forged = forged(&plugin, &person.session).await;
    forged.handle.answer(
        IntentId::mint(),
        id,
        picked("allow_once"),
        Activation::Programmatic,
    );
    let events = frames_to_the_mark(&host, &mut person).await;
    assert!(events.iter().any(|event| matches!(
        event,
        Event::InteractionResolved {
            by: ResolvedBy::Plugin { .. },
            ..
        }
    )));
    assert_eq!(pending.await.unwrap().unwrap(), picked("allow_once"));
    host.shutdown().await;
}

struct WeakHost(Arc<std::sync::Mutex<Option<std::sync::Weak<dyn HostApi>>>>);
#[async_trait]
impl Plugin for WeakHost {
    fn manifest(&self) -> &'static PluginManifest {
        &STANCE
    }
    fn register(&self, _: &mut Registrar) -> Result<(), PluginError> {
        Ok(())
    }
    async fn start(&self, host: HostHandle) -> Result<(), PluginError> {
        *self.0.lock().unwrap() = Some(Arc::downgrade(&host.0));
        Ok(())
    }
}
#[tokio::test]
async fn plugin_weak_handles_live_as_long_as_the_host_without_a_cycle() {
    let captured = Arc::new(std::sync::Mutex::new(None));
    let host = Host::build(
        vec![Box::new(WeakHost(captured.clone()))],
        HostConfig::new(env()),
    )
    .await
    .unwrap();
    let weak = captured.lock().unwrap().clone().unwrap();
    assert!(
        weak.upgrade().is_some(),
        "the plugin's weak host must survive start"
    );
    host.shutdown().await;
    drop(host);
    assert!(
        weak.upgrade().is_none(),
        "the adapter must not keep the host alive"
    );
}

#[tokio::test]
async fn the_plugin_host_preserves_repository_approval_checks() {
    let (host, plugin) = plugin_host().await;
    let repository = tempfile::tempdir().unwrap();
    let request = Approval {
        root: repository.path().into(),
        what: "acp.adapters:missing".into(),
    };
    let error = plugin.approve(request.clone()).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::PermissionDenied);
    assert_eq!(error.message, "approval requires person input");
    let error = bingo_sdk::render::under_input(true, plugin.approve(request))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert_eq!(error.message, "no repository entry with that name");
    host.shutdown().await;
}
