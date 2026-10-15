//! Explicit titles survive turns and journal revival. Auto-titling belongs to a mod.

use super::*;

fn named(title: Option<&str>) -> Mailbox {
    let head = SessionSummary {
        title: title.map(Into::into),
        ..summary("ses_1")
    };
    let provider = ScriptedProvider::new(vec![
        Script::Events(text("one")),
        Script::Events(text("two")),
    ]);
    spawn(head, None, Services::none(), |_| titled(provider, title))
}

/// The spec carries the name, as the host's fold of the journal does.
fn titled(provider: Arc<ScriptedProvider>, title: Option<&str>) -> Arc<TurnConfig> {
    let mut config = config(provider, vec![], Arc::new(NoHost));
    let mut spec = (*config.spec).clone();
    spec.title = title.map(Into::into);
    config.spec = Arc::new(spec);
    Arc::new(config)
}

fn renames(frames: &[Frame]) -> Vec<Option<String>> {
    frames
        .iter()
        .filter_map(|frame| match &frame.event {
            Event::SessionUpdated { summary } => Some(summary.title.clone()),
            _ => None,
        })
        .collect()
}

async fn ask(
    mailbox: &Mailbox,
    events: &mut FrameStream,
    state: &mut SessionState,
    said: &str,
) -> Vec<Frame> {
    mailbox.submit(IntentId::mint(), Input::text(said, Origin::surface("tui")));
    frames_until(events, state, turn_completed).await
}

#[tokio::test]
async fn a_session_somebody_named_is_never_renamed() {
    let mailbox = named(Some("reviewer"));
    let (mut state, mut events) = mailbox.attach().await.unwrap();
    let frames = ask(&mailbox, &mut events, &mut state, "review the diff").await;
    assert!(renames(&frames).is_empty(), "a seat keeps its name");
    assert_eq!(state.summary.title.as_deref(), Some("reviewer"));
}

/// The journal's own order decides: a name that landed after the first ask is
/// the session's name, and resuming it mints nothing over the top.
#[tokio::test]
async fn a_name_that_landed_after_the_first_ask_wins_on_resume() {
    let ts = jiff::Timestamp::from_second(0).unwrap();
    let frame = |seq: u64, event: Event| Frame {
        seq: Seq(seq),
        ts,
        session: SessionId::from_raw("ses_1"),
        cause: None,
        event,
    };
    let asked = Item {
        id: ItemId::from_raw("itm_1"),
        turn: None,
        round: 0,
        status: ItemStatus::Completed,
        started_at: ts,
        completed_at: Some(ts),
        intent: None,
        body: ItemBody::User {
            parts: vec![ContentPart::text("fix the parser")],
            origin: Origin::surface("tui"),
        },
        meta: Default::default(),
    };
    let frames = vec![
        frame(
            1,
            Event::SessionUpdated {
                summary: summary("ses_1"),
            },
        ),
        frame(2, Event::ItemCompleted { item: asked }),
        frame(
            3,
            Event::SessionUpdated {
                summary: SessionSummary {
                    title: Some("reviewer".into()),
                    history: bingo_sdk::SessionHistory {
                        messages: Some(1),
                        ..Default::default()
                    },
                    ..summary("ses_1")
                },
            },
        ),
    ];
    let provider = ScriptedProvider::new(vec![Script::Events(text("ok"))]);
    let mailbox = resume(frames, None, Services::none(), |_| {
        titled(provider, Some("reviewer"))
    })
    .unwrap();
    let (mut state, mut events) = mailbox.attach().await.unwrap();
    assert_eq!(
        state.summary.title.as_deref(),
        Some("reviewer"),
        "the head of the new segment carries the explicit name"
    );

    let frames = ask(&mailbox, &mut events, &mut state, "and the lexer").await;
    assert!(renames(&frames).is_empty());
    assert_eq!(state.summary.title.as_deref(), Some("reviewer"));
}
