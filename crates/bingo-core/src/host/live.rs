use super::*;

#[derive(Clone)]
pub(super) struct Live {
    pub(super) mailbox: Mailbox,
    pub(super) changing: Arc<tokio::sync::Mutex<()>>,
    pub(super) render: Arc<render_queue::Queue>,
    pub(super) render_started: tokio::time::Instant,
    pub(super) invalidations: Arc<Mutex<throttle::Throttle>>,
    pub(super) created_at: Timestamp,
    /// What the session was opened with, resolved: `/model` and `/think`
    /// rewrite it (ADR-0008 §4, ADR-0047 §1), and it is the one holder of
    /// the provider, the model and the level the next turn runs on.
    pub(super) spec: SessionSpec,
}

impl Live {
    pub(super) fn new(mailbox: Mailbox, summary: &SessionSummary, spec: SessionSpec) -> Self {
        Self {
            mailbox,
            changing: Arc::new(tokio::sync::Mutex::new(())),
            render: Arc::default(),
            render_started: tokio::time::Instant::now(),
            invalidations: Arc::default(),
            created_at: summary.created_at,
            spec,
        }
    }
}
