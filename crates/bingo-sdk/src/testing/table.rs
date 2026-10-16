use crate::*;
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

pub(super) struct Table {
    sessions: BTreeMap<SessionId, SessionSummary>,
    next: u64,
}

impl Table {
    pub fn new(sessions: Vec<SessionSummary>) -> Self {
        Self {
            sessions: sessions
                .into_iter()
                .map(|summary| (summary.id.clone(), summary))
                .collect(),
            next: 0,
        }
    }

    pub fn sessions(&self, filter: SessionFilter) -> Result<Vec<SessionSummary>, KernelError> {
        let mut summaries: Vec<_> = self
            .sessions
            .values()
            .filter(|summary| {
                filter
                    .cwd
                    .as_ref()
                    .is_none_or(|cwd| Path::new(&summary.cwd) == cwd)
            })
            .filter(|summary| {
                filter.parent.as_ref().is_none_or(|parent| {
                    summary
                        .parent
                        .as_ref()
                        .is_some_and(|link| &link.session == parent)
                })
            })
            .cloned()
            .collect();
        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then(a.id.cmp(&b.id)));
        summaries.truncate(filter.limit.unwrap_or(usize::MAX));
        Ok(summaries)
    }

    pub fn open(&mut self, selector: SessionSelector) -> Result<Attachment, KernelError> {
        let summary = match selector {
            SessionSelector::Create { spec } => self.create(spec),
            SessionSelector::ById { id } => self.sessions.get(&id).cloned().ok_or_else(missing),
            SessionSelector::ByKey { key } => self
                .sessions
                .values()
                .find(|summary| summary.key.as_ref() == Some(&key))
                .cloned()
                .ok_or_else(missing),
            SessionSelector::Latest { cwd } => self
                .sessions(SessionFilter {
                    cwd: Some(cwd),
                    limit: Some(1),
                    ..SessionFilter::default()
                })?
                .into_iter()
                .next()
                .ok_or_else(missing),
        }?;
        Ok(Attachment {
            session: summary.id.clone(),
            snapshot: SessionState::new(summary),
            history: None,
            events: Box::pin(futures::stream::empty()),
            handle: SessionHandle(Arc::new(SnapshotPort)),
        })
    }

    fn create(&mut self, spec: SessionSpec) -> Result<SessionSummary, KernelError> {
        if spec.key.as_ref().is_some_and(|key| {
            self.sessions
                .values()
                .any(|summary| summary.key.as_ref() == Some(key))
        }) {
            return Err(KernelError::new(
                ErrorCode::SessionLocked,
                "session key is already taken",
            ));
        }
        let id = self.next_id();
        let summary = SessionSummary {
            choice: crate::SessionModel {
                model: spec.model,
                model_scope: None,
                effort: None,
            },
            history: crate::SessionHistory {
                first_ask: None,
                forked_from: None,
                messages: Some(0),
            },
            id,
            key: spec.key,
            title: spec.title,
            cwd: spec.cwd.to_string_lossy().into_owned(),
            parent: spec.parent,
            driver: spec.driver,
            provider: spec.provider,
            system_extra: spec.system_extra,
            tools: spec.tools,
            created_at: jiff::Timestamp::UNIX_EPOCH,
            updated_at: jiff::Timestamp::UNIX_EPOCH,
            usage: Usage::default(),
            busy: false,
            asking: false,
        };
        self.sessions.insert(summary.id.clone(), summary.clone());
        Ok(summary)
    }

    fn next_id(&mut self) -> SessionId {
        loop {
            self.next += 1;
            let id = SessionId::from_raw(format!("ses_test_{}", self.next));
            if !self.sessions.contains_key(&id) {
                return id;
            }
        }
    }

    pub fn close(&self, session: &SessionId, _: CloseReason) -> Result<(), KernelError> {
        self.sessions.get(session).map(|_| ()).ok_or_else(missing)
    }

    pub fn delete(&mut self, session: &SessionId) -> Result<(), KernelError> {
        self.sessions
            .remove(session)
            .map(|_| ())
            .ok_or_else(missing)
    }
}

fn missing() -> KernelError {
    KernelError::new(ErrorCode::SessionNotFound, "no such test session")
}

/// The table only serves snapshots. Script `open` to supply an active port.
struct SnapshotPort;

#[async_trait]
impl SessionPort for SnapshotPort {
    fn submit(&self, _: IntentId, _: Input) {}
    fn interrupt(&self, _: IntentId, _: InterruptScope) {}
    fn answer(&self, _: IntentId, _: InteractionId, _: Answer, _: Activation) {}
    async fn history(&self, _: HistoryPage) -> Result<HistoryChunk, KernelError> {
        Err(KernelError::new(
            ErrorCode::InvalidInput,
            "this test session has no history",
        ))
    }
    async fn events_since(&self, _: Seq) -> Result<FrameStream, KernelError> {
        Ok(Box::pin(futures::stream::empty()))
    }
}
