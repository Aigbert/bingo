//! The doubles the loop is driven against: a host and a session that answer
//! from a script and remember what was written to them, and a screen that
//! keeps what it was asked to paint — the bytes included, because the bell,
//! the title, a selection and a notification never paint a cell.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use bingo_sdk::{
    Activation, ArgSpec, Attachment, Catalog, CatalogEntry, CatalogKind, CommandSpec, FrameStream,
    GatewayEvent, HistoryChunk, HistoryPage, HostHandle, Input, IntentId, InterruptScope,
    KernelError, Seq, SessionHandle, SessionId, SessionPort, SessionSelector, SessionState,
    SessionSummary,
};
use futures::StreamExt;
use serde_json::Value;

use crate::terminal::Screen;
use crate::test_support::{child_id, child_summary, state, summary};

/// A session that hands back a scripted frame list and remembers the writes.
#[derive(Debug, Default)]
pub struct TestSession {
    frames: Vec<bingo_sdk::Frame>,
    /// How long each frame waits before it arrives, so a test can be a storm
    /// rather than a burst.
    pace: std::time::Duration,
    submitted: Mutex<Vec<(IntentId, Input)>>,
    answers: Mutex<Vec<(bingo_sdk::InteractionId, bingo_sdk::Answer, Activation)>>,
    interrupts: Mutex<Vec<(IntentId, InterruptScope)>>,
}

impl TestSession {
    fn live(&self) -> FrameStream {
        let frames = self.frames.clone();
        if self.pace.is_zero() {
            return Box::pin(futures::stream::iter(frames));
        }
        let pace = self.pace;
        Box::pin(futures::stream::iter(frames).then(move |frame| async move {
            tokio::time::sleep(pace).await;
            frame
        }))
    }

    pub fn submitted(&self) -> Vec<Input> {
        self.submissions()
            .into_iter()
            .map(|(_, input)| input)
            .collect()
    }

    pub fn submissions(&self) -> Vec<(IntentId, Input)> {
        self.submitted.lock().expect("no poisoned lock").clone()
    }

    pub fn answers(&self) -> Vec<(bingo_sdk::InteractionId, bingo_sdk::Answer, Activation)> {
        self.answers.lock().expect("no poisoned lock").clone()
    }

    pub fn interrupts(&self) -> usize {
        self.interrupts.lock().expect("no poisoned lock").len()
    }
    pub fn interruptions(&self) -> Vec<(IntentId, InterruptScope)> {
        self.interrupts.lock().expect("no poisoned lock").clone()
    }
}

#[async_trait]
impl SessionPort for TestSession {
    fn submit(&self, intent: IntentId, input: Input) {
        self.submitted
            .lock()
            .expect("no poisoned lock")
            .push((intent, input));
    }

    fn interrupt(&self, intent: IntentId, scope: InterruptScope) {
        self.interrupts
            .lock()
            .expect("no poisoned lock")
            .push((intent, scope));
    }

    fn answer(
        &self,
        _intent: IntentId,
        interaction: bingo_sdk::InteractionId,
        answer: bingo_sdk::Answer,
        activation: Activation,
    ) {
        self.answers
            .lock()
            .expect("no poisoned lock")
            .push((interaction, answer, activation));
    }

    async fn history(&self, _page: HistoryPage) -> Result<HistoryChunk, KernelError> {
        Ok(HistoryChunk {
            items: Vec::new(),
            next: None,
            generation: 0,
            oversized: None,
        })
    }

    /// The journal replay: durable frames only, like the kernel's.
    async fn events_since(&self, since: Seq) -> Result<FrameStream, KernelError> {
        let frames: Vec<_> = self
            .frames
            .iter()
            .filter(|f| f.seq > since && f.event.is_durable())
            .cloned()
            .collect();
        Ok(Box::pin(futures::stream::iter(frames)))
    }
}

#[derive(Debug)]
pub struct TestHost {
    session: Arc<TestSession>,
    /// The mailbox `open(ById)` hands out for the child in the tree.
    child: Arc<TestSession>,
    /// What `sessions` answers with beyond the root: what the store holds.
    stored: Vec<SessionSummary>,
    closed: Mutex<Vec<SessionId>>,
    /// What it announces on its gateway, taken by the first subscriber.
    announcements: Mutex<Vec<GatewayEvent>>,
    /// Set as each announcement is handed over: a host that has said a
    /// catalogue changed answers the next read with the model it gained.
    announced: Arc<AtomicBool>,
}

impl TestHost {
    pub fn with(frames: Vec<bingo_sdk::Frame>) -> (HostHandle, Arc<TestSession>) {
        let (host, session, _) = Self::tree(frames);
        (host, session)
    }

    /// A host whose frames arrive `pace` apart: what a storm of deltas looks
    /// like from the loop's side.
    pub fn paced(
        frames: Vec<bingo_sdk::Frame>,
        pace: std::time::Duration,
    ) -> (HostHandle, Arc<TestSession>) {
        let session = scripted(frames, pace);
        let host = Self::over(Arc::clone(&session), Vec::new(), Vec::new());
        (Arc::new(host).scripted().handle(), session)
    }

    /// The root's mailbox and the child's, which `open(ById)` answers with.
    pub fn tree(frames: Vec<bingo_sdk::Frame>) -> (HostHandle, Arc<TestSession>, Arc<TestSession>) {
        let session = scripted(frames, std::time::Duration::ZERO);
        let host = Self::over(Arc::clone(&session), Vec::new(), Vec::new());
        let child = Arc::clone(&host.child);
        (Arc::new(host).scripted().handle(), session, child)
    }

    pub fn with_stored(
        frames: Vec<bingo_sdk::Frame>,
        stored: Vec<SessionSummary>,
    ) -> (HostHandle, Arc<TestSession>) {
        let session = scripted(frames, std::time::Duration::ZERO);
        let host = Self::over(Arc::clone(&session), stored, Vec::new());
        (Arc::new(host).scripted().handle(), session)
    }

    fn over(
        session: Arc<TestSession>,
        stored: Vec<SessionSummary>,
        announcements: Vec<GatewayEvent>,
    ) -> TestHost {
        TestHost {
            session,
            child: Arc::new(TestSession::default()),
            stored,
            closed: Mutex::new(Vec::new()),
            announcements: Mutex::new(announcements),
            announced: Arc::default(),
        }
    }
}

fn scripted(frames: Vec<bingo_sdk::Frame>, pace: std::time::Duration) -> Arc<TestSession> {
    Arc::new(TestSession {
        frames,
        pace,
        ..Default::default()
    })
}

impl TestHost {
    fn scripted(self: &Arc<Self>) -> bingo_sdk::testing::TestHost {
        let host = bingo_sdk::testing::TestHost::default();
        self.script_sessions(&host);
        self.script_open(&host);
        self.script_close(&host);
        host.on_delete(|_session| async move { Ok(()) });
        host.on_deliver(|_to, _intent, _input, _delivery| async move {
            unreachable!("this double delivers nothing")
        });
        host.on_extend(|_session, _plugin, _kind, _payload| async move {
            unreachable!("this double extends nothing")
        });
        host.on_signal(|_session, _plugin, _kind, _payload| async move {
            unreachable!("this double signals nothing")
        });
        self.script_catalog(&host);
        self.script_gateway_events(&host);
        host
    }

    fn script_sessions(self: &Arc<Self>, host: &bingo_sdk::testing::TestHost) {
        let fixture = self.clone();
        host.on_sessions(move |_filter| {
            let fixture = fixture.clone();
            async move {
                let mut all = vec![summary()];
                all.extend(fixture.stored.iter().cloned());
                Ok(all)
            }
        });
    }

    fn script_open(self: &Arc<Self>, host: &bingo_sdk::testing::TestHost) {
        let fixture = self.clone();
        host.on_open(move |selector, _who, _options| {
            let fixture = fixture.clone();
            async move {
                if matches!(&selector, SessionSelector::ById { id } if id == &child_id()) {
                    return Ok(Attachment {
                        session: child_id(),
                        snapshot: SessionState::new(child_summary("reviewer")),
                        history: None,
                        events: Box::pin(futures::stream::empty()),
                        handle: SessionHandle(Arc::clone(&fixture.child) as Arc<dyn SessionPort>),
                    });
                }
                Ok(Attachment {
                    session: SessionId::from_raw("ses_1"),
                    snapshot: state(),
                    history: None,
                    events: fixture.session.live(),
                    handle: SessionHandle(Arc::clone(&fixture.session) as Arc<dyn SessionPort>),
                })
            }
        });
    }

    fn script_close(self: &Arc<Self>, host: &bingo_sdk::testing::TestHost) {
        let fixture = self.clone();
        host.on_close(move |session, _reason| {
            let fixture = fixture.clone();
            async move {
                let session = &session;
                fixture
                    .closed
                    .lock()
                    .expect("no poisoned lock")
                    .push(session.clone());
                Ok(())
            }
        });
    }

    fn script_catalog(self: &Arc<Self>, host: &bingo_sdk::testing::TestHost) {
        let fixture = self.clone();
        host.on_catalog(move |kind| {
            let fixture = fixture.clone();
            async move {
                let entries = match kind {
                    CatalogKind::Commands => vec![CatalogEntry {
                        id: "model".into(),
                        label: "model".into(),
                        meta: serde_json::to_value(CommandSpec {
                            opens: false,
                            keys: vec![],
                            hidden: false,
                            name: "model".into(),
                            aliases: vec![],
                            hint: "[provider/]model".into(),
                            args: ArgSpec::Catalog {
                                source: "models".into(),
                            },
                            instant: true,
                            family: "kernel".into(),
                        })
                        .expect("a serialisable spec"),
                    }],
                    // A second model appears once this host has announced that the
                    // catalogue changed: a refresh, as a client sees one.
                    _ => ["fake/fake-1"]
                        .into_iter()
                        .chain(
                            fixture
                                .announced
                                .load(Ordering::SeqCst)
                                .then_some("fake/fake-2"),
                        )
                        .map(|id| CatalogEntry {
                            id: id.into(),
                            label: id.split('/').next_back().unwrap_or(id).into(),
                            meta: Value::Null,
                        })
                        .collect(),
                };
                Ok(Catalog { kind, entries })
            }
        });
    }

    fn script_gateway_events(self: &Arc<Self>, host: &bingo_sdk::testing::TestHost) {
        let fixture = self.clone();
        host.on_gateway_events(move || {
            let announcements =
                std::mem::take(&mut *fixture.announcements.lock().expect("no poisoned lock"));
            let announced = Arc::clone(&fixture.announced);
            Box::pin(futures::stream::iter(announcements).map(move |event| {
                announced.store(true, Ordering::SeqCst);
                event
            }))
        });
    }
}

/// A screen that keeps what it was asked to paint.
#[derive(Debug, Default)]
pub struct Recorder {
    pub motion: Option<bool>,
    pub frames: Vec<String>,
    pub painted_at: Vec<std::time::Instant>,
    pub titles: Vec<String>,
    pub bells: usize,
    /// The bytes handed to the terminal's clipboard, verbatim.
    pub copies: Vec<Vec<u8>>,
    /// The bytes sent to the desktop, verbatim.
    pub notifications: Vec<Vec<u8>>,
    /// The bytes the pictures of a frame were made of, verbatim.
    pub places: Vec<Vec<u8>>,
    /// The questions put to the terminal, verbatim.
    pub asks: Vec<Vec<u8>>,
}

impl Recorder {
    pub fn last(&self) -> &str {
        self.frames.last().map(String::as_str).unwrap_or_default()
    }
}

impl Screen for Recorder {
    fn detection(&mut self) -> crate::render::Detection {
        let mut detection = crate::render::Detection::default();
        if let Some(motion) = self.motion {
            detection.motion = motion;
        }
        detection
    }
    fn draw(
        &mut self,
        layout: &crate::layout::Layout,
        render: &crate::render::Render,
    ) -> std::io::Result<()> {
        let area = layout.canvas.buffer.area;
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(area.width, area.height))
                .expect("test terminal");
        terminal
            .draw(|frame| crate::layout::paint(layout, frame, render))
            .expect("paint");
        self.frames.push(terminal.backend().to_string());
        self.painted_at.push(tokio::time::Instant::now().into_std());
        Ok(())
    }

    fn title(&mut self, text: &str) -> std::io::Result<()> {
        if self.titles.last().map(String::as_str) != Some(text) {
            self.titles.push(text.to_string());
        }
        Ok(())
    }

    fn bell(&mut self) -> std::io::Result<()> {
        self.bells += 1;
        Ok(())
    }

    fn notify(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.notifications.push(bytes.to_vec());
        Ok(())
    }

    fn copy(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.copies.push(bytes.to_vec());
        Ok(())
    }

    fn place(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.places.push(bytes.to_vec());
        Ok(())
    }

    fn ask(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.asks.push(bytes.to_vec());
        Ok(())
    }

    fn size(&self) -> (u16, u16) {
        (80, 24)
    }
}

/// The options a surface is handed, pointed at a scratch directory.
pub fn options(prompt: Option<&str>, home: &std::path::Path) -> bingo_sdk::SurfaceOptions {
    bingo_sdk::SurfaceOptions {
        cwd: home.to_path_buf(),
        selector: SessionSelector::Create {
            spec: Default::default(),
        },
        prompt: prompt.map(str::to_owned),

        env: Arc::new(bingo_sdk::Env::rooted(home)),
    }
}

/// A key stream that yields its script and then keeps waiting, as a real
/// terminal does. Each press is held back a moment so the frames already on
/// the stream are folded first, which is the order a person would see.
pub fn keys(script: Vec<crossterm::event::KeyEvent>) -> crate::run::Keys {
    pressed(script, std::time::Duration::from_millis(5))
}

/// The same, with a wait before every press: under `tokio::time::pause` it
/// is time the loop must sit through with nothing to do.
pub fn keys_after(
    wait: std::time::Duration,
    script: Vec<crossterm::event::KeyEvent>,
) -> crate::run::Keys {
    pressed(script, wait)
}

fn pressed(script: Vec<crossterm::event::KeyEvent>, wait: std::time::Duration) -> crate::run::Keys {
    terminal_events(
        script
            .into_iter()
            .map(crossterm::event::Event::Key)
            .collect(),
        wait,
    )
}

/// Everything else a terminal sends: the window taking the focus and losing
/// it, which is what turns a completion into a notification (§6).
pub fn terminal_events(
    script: Vec<crossterm::event::Event>,
    wait: std::time::Duration,
) -> crate::run::Keys {
    let events = futures::stream::iter(script).then(move |event| async move {
        tokio::time::sleep(wait).await;
        event
    });
    Box::pin(events.chain(futures::stream::pending()))
}
