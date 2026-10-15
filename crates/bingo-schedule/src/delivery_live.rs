//! A host whose sessions are live: every line a fire hands one — submitted
//! or delivered — reaches the test, and the test answers it when it likes.

use async_trait::async_trait;
use bingo_sdk::{
    Activation, Answer, Attachment, Delivery, Event, Frame, FrameStream, HistoryChunk, HistoryPage,
    Input, IntentId, IntentOutcome, InteractionId, InterruptScope, KernelError, Seq, SessionHandle,
    SessionId, SessionPort, testing::TestHost,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

type Lines = UnboundedSender<(IntentId, Input)>;
type Answers = Arc<Mutex<Option<UnboundedSender<Frame>>>>;

pub(super) struct Live {
    pub host: TestHost,
    /// Every line handed to a session, in order.
    pub lines: UnboundedReceiver<(IntentId, Input)>,
    /// How many sessions opened.
    pub opened: Arc<AtomicUsize>,
    answers: Answers,
}

impl Live {
    /// A live host whose first `panics` opens panic.
    pub fn new(panics: usize) -> Self {
        let host = TestHost::with_sessions(vec![]);
        let (sent, lines) = unbounded_channel();
        let live = Self {
            host,
            lines,
            opened: Arc::new(AtomicUsize::new(0)),
            answers: Arc::new(Mutex::new(None)),
        };
        live.script_open(panics, sent.clone());
        live.host.on_deliver(move |_, intent, input, delivery| {
            assert_eq!(delivery, Delivery::Wake, "a fire is a wake");
            sent.send((intent, input)).expect("the test is listening");
            async { Ok(()) }
        });
        live.host.on_notice(|_, _, _| async { Ok(()) });
        live
    }

    fn script_open(&self, panics: usize, sent: Lines) {
        let table = TestHost::with_sessions(vec![]);
        let tried = Arc::new(AtomicUsize::new(0));
        let opened = self.opened.clone();
        let answers = self.answers.clone();
        self.host.on_open(move |selector, who, options| {
            let (table, sent, opened, answers) = (
                table.handle(),
                sent.clone(),
                opened.clone(),
                answers.clone(),
            );
            let panicking = tried.fetch_add(1, Ordering::SeqCst) < panics;
            async move {
                assert!(!panicking, "the fixture's open panics");
                let mut attachment = table.open(selector, who, options).await?;
                let (frames, events) = unbounded_channel();
                *answers.lock().expect("answers") = Some(frames);
                attachment.events = stream(events);
                attachment.handle = SessionHandle(Arc::new(Port { sent }));
                opened.fetch_add(1, Ordering::SeqCst);
                Ok::<Attachment, KernelError>(attachment)
            }
        });
    }

    /// The open session answers `intent`. A fire that stopped listening
    /// drops the answer, as a real session's stream would.
    pub fn answer(&self, intent: &IntentId, outcome: IntentOutcome) {
        let frame = Frame {
            seq: Seq(1),
            ts: jiff::Timestamp::UNIX_EPOCH,
            session: SessionId::from_raw("ses_live"),
            cause: Some(intent.clone()),
            event: Event::IntentAck {
                intent: intent.clone(),
                outcome,
            },
        };
        let answers = self.answers.lock().expect("answers");
        let open = answers.as_ref().expect("a session is open");
        let _unheard = open.send(frame);
    }

    pub fn opened(&self) -> usize {
        self.opened.load(Ordering::SeqCst)
    }
}

fn stream(mut events: UnboundedReceiver<Frame>) -> FrameStream {
    Box::pin(futures::stream::poll_fn(move |cx| events.poll_recv(cx)))
}

struct Port {
    sent: Lines,
}

#[async_trait]
impl SessionPort for Port {
    fn submit(&self, intent: IntentId, input: Input) {
        self.sent
            .send((intent, input))
            .expect("the test is listening");
    }
    fn interrupt(&self, _: IntentId, _: InterruptScope) {}
    fn answer(&self, _: IntentId, _: InteractionId, _: Answer, _: Activation) {}
    async fn history(&self, _: HistoryPage) -> Result<HistoryChunk, KernelError> {
        Err(KernelError::new(
            bingo_sdk::ErrorCode::InvalidInput,
            "no history here",
        ))
    }
    async fn events_since(&self, _: Seq) -> Result<FrameStream, KernelError> {
        Ok(Box::pin(futures::stream::pending()))
    }
}
