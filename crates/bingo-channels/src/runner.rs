//! One conversation, on one session, for as long as the surface runs.
//!
//! It is a client like every other: it folds frames with `SessionState::apply`
//! and derives what to say from the fold (ADR-0002). What it adds is the other
//! direction — a reply that answers an open question resolves it, and anything
//! else is a prompt.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use bingo_sdk::{
    Activation, Answer, Applied, Attachment, CancelReason, ClientIdentity, Delivery, ErrorCode,
    Event, Frame, FrameStream, HostHandle, Image, Input, IntentId, InteractionId, KernelError,
    OpenOptions, Origin, SessionChange, SessionHandle, SessionSelector, SessionSpec, TreeState,
};
use futures::StreamExt;
use tokio::sync::mpsc;

use crate::adapter::{ChannelAdapter, ChannelCommand, Incoming, Mark, Mode, Outcome};
use crate::conversation::{Conversation, Posted};
use crate::deliver::{Deliverer, Op};
use crate::directory::{Directory, Seat};
use crate::error::ChannelError;
use crate::gate::Gate;
use crate::question::{Question, Settled, withdrawn};

/// The surface id, and the `Origin.surface` of everything a chat submits.
pub const SURFACE_ID: &str = "channels";

/// A question this conversation is showing, and how it was drawn — which is
/// what decides how the buttons come off again.
struct Asked {
    question: Question,
    at: Posted,
    native: bool,
}

mod drawing;

pub struct Runner {
    menu_changes: Option<tokio::sync::watch::Sender<bool>>,
    host: HostHandle,
    adapter: Arc<dyn ChannelAdapter>,
    conversation: Conversation,
    key: String,
    cwd: std::path::PathBuf,
    /// One reducer per session in the tree, the root's from the snapshot.
    tree: TreeState,
    events: FrameStream,
    handle: SessionHandle,
    deliverer: Deliverer,
    /// The message the answer is streaming into, while there is one.
    streaming: Option<Posted>,
    /// Where this conversation is, for whoever else needs to reach it — the
    /// tool that posts a file into it (ADR-0051 §3). The message a reply hangs
    /// under lives in the seat, so this runner and that tool cannot disagree
    /// about which message that is.
    directory: Directory,
    /// The message being worked on and the sign that says so, while a turn
    /// this chat started is running (ADR-0051 §5).
    working: Option<(Posted, Mark)>,
    asked: BTreeMap<InteractionId, Asked>,
    drawn: VecDeque<(bingo_channel_adapter::drawing::Drawing, Posted)>,
    inbound: mpsc::Receiver<Incoming>,
}

impl Runner {
    /// Open or continue this conversation's session, keyed `<adapter>/<chat>
    /// [/<thread>]` — a key only this plugin mints (ADR-0016 §4).
    pub async fn open(
        host: &HostHandle,
        adapter: Arc<dyn ChannelAdapter>,
        conversation: Conversation,
        cwd: std::path::PathBuf,
        gate: Gate,
        directory: Directory,
        inbound: mpsc::Receiver<Incoming>,
    ) -> Result<Self, KernelError> {
        let key = format!("{}/{}", adapter.id(), conversation.path());
        let attachment = attach(host, &key, cwd.clone()).await?;
        let Attachment {
            session,
            snapshot,
            events,
            handle,
            ..
        } = attachment;
        let deliverer = Deliverer::new(adapter.limits().clone(), gate, key.clone());
        directory.sit(
            session.clone(),
            Seat {
                adapter: Arc::clone(&adapter),
                conversation: conversation.clone(),
                parent: None,
            },
        );
        Ok(Self {
            menu_changes: None,
            host: host.clone(),
            adapter,
            conversation,
            cwd,
            tree: TreeState::new(snapshot),
            events,
            handle,
            deliverer,
            streaming: None,
            directory,
            working: None,
            asked: BTreeMap::new(),
            drawn: VecDeque::new(),
            key,
            inbound,
        })
    }

    pub(crate) fn menus(mut self, changes: tokio::sync::watch::Sender<bool>) -> Self {
        self.menu_changes = Some(changes);
        self
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    /// Frames out, replies in, until the session's stream ends or the chat
    /// goes away — and then this conversation is nobody's to reach.
    pub async fn run(mut self) {
        self.pump().await;
        self.directory.leave(&self.tree.root().summary.id);
    }

    async fn pump(&mut self) {
        loop {
            let due = self.deliverer.due();
            tokio::select! {
                frame = self.events.next() => match frame {
                    Some(frame) => self.saw(&frame).await,
                    None => return,
                },
                _ = tick(due) => {
                    let ops = self.deliverer.tick(Instant::now());
                    self.perform(ops).await;
                }
                event = self.inbound.recv() => match event {
                    Some(event) => self.heard(event).await,
                    None => return,
                },
            }
        }
    }

    async fn saw(&mut self, frame: &Frame) {
        if self.tree.apply(frame) == Applied::Stale {
            return;
        }
        if !self.concerns_this_chat(frame) {
            return;
        }
        if (matches!(&frame.event, Event::CatalogChanged { kind } if kind == "commands")
            || matches!(frame.event, Event::ConfigChanged { .. }))
            && let Some(changes) = &self.menu_changes
        {
            changes.send_modify(|value| *value = !*value);
        }
        let Some(state) = self.tree.get(&frame.session) else {
            return;
        };
        let input = match &frame.event {
            Event::IntentAck { intent, .. } => self.deliverer.input(intent).cloned(),
            _ => None,
        };
        let ops = self
            .deliverer
            .apply(frame, state, Instant::now())
            .into_iter()
            .chain(crate::sites::action_notice(frame, state, input.as_ref()))
            .map(
                |op| match crate::sites::request(frame, state, &op, input.as_ref()) {
                    Some(request) => Op::Site {
                        request,
                        fallback: Box::new(op),
                    },
                    None => op,
                },
            )
            .collect();
        self.perform(ops).await;
        if let Event::Invalidated { requests } = &frame.event {
            self.redraw(requests).await;
        }
    }

    /// A sub-session's prose is its parent's to report; its questions are
    /// nobody else's, so those reach the chat (ADR-0010 §3).
    fn concerns_this_chat(&self, frame: &Frame) -> bool {
        self.tree.get(&frame.session).is_some()
            && (frame.session == self.tree.root().summary.id
                || matches!(
                    frame.event,
                    Event::InteractionOpened { .. }
                        | Event::InteractionResolved { .. }
                        | Event::InteractionCancelled { .. }
                ))
    }

    async fn perform(&mut self, ops: Vec<Op>) {
        for op in ops {
            if let Err(error) = self.deliver(op).await {
                // A platform that refuses one message has not ended the
                // conversation; the next frame is still worth delivering.
                tracing::warn!(%error, key = %self.key, "the chat refused a message");
            }
        }
    }

    async fn deliver(&mut self, op: Op) -> Result<(), ChannelError> {
        match op {
            Op::Site { request, fallback } => self.draw_site(request, *fallback).await,
            Op::Open => self.begin().await,
            Op::Replace { full } => self.replace(&full).await,
            Op::Finalize { text, question } => self.finalize(&text, question).await,
            Op::Status { text } => self.post(&text).await.map(drop),
            Op::Card { text } => self.card(&text).await,
            Op::Resolved { question, outcome } => self.settle(&question, &outcome).await,
            Op::Ended { failed } => {
                self.ended(failed).await;
                Ok(())
            }
        }
    }

    /// Say that the message just submitted is being worked on, where the
    /// platform has a way to say it.
    ///
    /// One sign at a time: a second message arriving mid-turn would strand
    /// the first sign with nothing left holding its mark, and the sign
    /// already up says exactly what the second one would.
    async fn acknowledge(&mut self) {
        if self.working.is_some() {
            return;
        }
        let Some(at) = self.directory.parent(&self.tree.root().summary.id) else {
            return;
        };
        let adapter = Arc::clone(&self.adapter);
        let Some(sign) = adapter.acknowledge() else {
            return;
        };
        match sign.begin(&at).await {
            Ok(mark) => self.working = Some((at, mark)),
            Err(error) => {
                tracing::warn!(%error, key = %self.key, "the chat could not say it was working");
            }
        }
    }

    /// The sign comes off however the turn went. A platform that will not
    /// take it off has not failed the turn and has not lost the answer, so it
    /// costs a warning and nothing else.
    async fn ended(&mut self, failed: bool) {
        let Some((at, mark)) = self.working.take() else {
            return;
        };
        let adapter = Arc::clone(&self.adapter);
        let Some(sign) = adapter.acknowledge() else {
            return;
        };
        let outcome = match failed {
            true => Outcome::Failed,
            false => Outcome::Done,
        };
        if let Err(error) = sign.end(&at, mark, outcome).await {
            tracing::warn!(%error, key = %self.key, "the chat kept the sign it was working");
        }
    }

    /// A message the answer streams into — only where there is something to
    /// stream with. Without an `edit()` the answer arrives whole and late, so
    /// the typing affordance is what a person gets meanwhile; a platform that
    /// streams needs none, because the message writing itself is the sign.
    async fn begin(&mut self) -> Result<(), ChannelError> {
        if self.adapter.edit().is_none() {
            return match self.adapter.typing() {
                Some(typing) => typing.poke(&self.conversation).await,
                None => Ok(()),
            };
        }
        self.streaming = Some(self.post_mode("", Mode::Stream).await?);
        Ok(())
    }

    async fn replace(&mut self, full: &str) -> Result<(), ChannelError> {
        match (self.adapter.edit(), &self.streaming) {
            (Some(edit), Some(at)) => edit.replace(at, full).await,
            _ => Ok(()),
        }
    }

    /// The answer, and then the question that stopped it.
    ///
    /// The two are separate deliveries and the second never depends on the
    /// first. A platform that refused the answer has not refused the question,
    /// and a question that never arrives is a turn that never ends: the
    /// session waits on an interaction nobody was ever shown, and every later
    /// message queues behind it. That is the whole "it worked for a few
    /// messages and then stuck" failure, so both are attempted and the first
    /// error is what is reported.
    async fn finalize(
        &mut self,
        text: &str,
        question: Option<Question>,
    ) -> Result<(), ChannelError> {
        let answer = self.adapter.final_answer(text, self.tree.root());
        let said = self.say(&answer).await;
        let asked = match question {
            Some(question) => self.ask(question).await,
            None => Ok(()),
        };
        said.and(asked)
    }

    /// The answer itself: finished into the message it was streaming into,
    /// else posted whole.
    ///
    /// A stream that will not close is not an answer that is gone. The card
    /// may have been auto-closed by the platform under a long turn, or the
    /// sequence refused; either way the text still exists and goes out as its
    /// own message rather than disappearing with the card.
    async fn say(&mut self, text: &str) -> Result<(), ChannelError> {
        let finished = match (self.adapter.edit(), self.streaming.take()) {
            (Some(edit), Some(at)) => Some(edit.finish(&at, text).await),
            _ => None,
        };
        match finished {
            Some(Ok(())) | None if text.is_empty() => Ok(()),
            Some(Ok(())) => Ok(()),
            Some(Err(error)) if !text.is_empty() => {
                tracing::warn!(
                    %error,
                    key = %self.key,
                    "the streamed message would not close; posting the answer whole"
                );
                self.post(text).await.map(drop)
            }
            Some(Err(error)) => Err(error),
            None => self.post(text).await.map(drop),
        }
    }

    /// A question is its own message, never buttons on a live stream: a
    /// platform that is mid-stream will not deliver a callback (ADR-0016 §6).
    async fn ask(&mut self, question: Question) -> Result<(), ChannelError> {
        let native = self
            .adapter
            .buttons()
            .filter(|_| question.buttons(self.adapter.limits()).is_some());
        // Buttons are an affordance, not the question. A platform that refused
        // the card can still be asked in words, and losing the question here
        // would leave the session waiting for an answer nobody was shown.
        let posted = match native {
            Some(buttons) => match buttons.ask(&self.conversation, &question).await {
                Ok(at) => Ok((at, true)),
                Err(error) => {
                    tracing::warn!(
                        %error,
                        key = %self.key,
                        "the question's buttons were refused; asking in words"
                    );
                    self.post(&question.numbered()).await.map(|at| (at, false))
                }
            },
            None => self.post(&question.numbered()).await.map(|at| (at, false)),
        };
        let (at, native) = posted?;
        self.asked.insert(
            question.id.clone(),
            Asked {
                question,
                at,
                native,
            },
        );
        Ok(())
    }

    /// The buttons come off however they went on, and the outcome goes where
    /// they were. A question drawn as a numbered list has no live button to
    /// strip, so if the platform will not edit that message the outcome is
    /// said in a new one rather than lost.
    async fn settle(&mut self, id: &InteractionId, outcome: &str) -> Result<(), ChannelError> {
        let Some(asked) = self.asked.remove(id) else {
            return Ok(());
        };
        if asked.native
            && let Some(buttons) = self.adapter.buttons()
        {
            return buttons.settle(&asked.at, &asked.question, outcome).await;
        }
        let settled = format!("{}\n\n{outcome}", asked.question.prompt);
        let edited = match self.adapter.edit() {
            Some(edit) => edit.replace(&asked.at, &settled).await,
            None => Err(ChannelError::Unsupported("editing")),
        };
        match edited {
            Ok(()) => Ok(()),
            Err(_) => self.post(&settled).await.map(drop),
        }
    }

    fn post(
        &self,
        text: &str,
    ) -> impl std::future::Future<Output = Result<Posted, ChannelError>> + Send + use<> {
        self.post_mode(text, Mode::Once)
    }

    /// A command's result: its own message, shown the way an answer is. A
    /// platform with a card draws one, so a table stays a table; one without
    /// gets the same text. The deliverer has already applied its limits.
    async fn card(&mut self, text: &str) -> Result<(), ChannelError> {
        if text.is_empty() {
            return Ok(());
        }
        let adapter = Arc::clone(&self.adapter);
        let Some(edit) = adapter.edit() else {
            return self.post(text).await.map(drop);
        };
        let at = self.post_mode(text, Mode::Stream).await?;
        if let Err(error) = edit.finish(&at, text).await {
            // A card that will not close is not a result that is gone.
            tracing::warn!(
                %error,
                key = %self.key,
                "a command's card would not close; posting it whole"
            );
            return self.post(text).await.map(drop);
        }
        Ok(())
    }

    fn post_mode(
        &self,
        text: &str,
        mode: Mode,
    ) -> impl std::future::Future<Output = Result<Posted, ChannelError>> + Send + use<> {
        post(
            Arc::clone(&self.adapter),
            self.conversation.clone(),
            self.directory.parent(&self.tree.root().summary.id),
            text.to_string(),
            mode,
        )
    }

    async fn heard(&mut self, event: Incoming) {
        match event {
            Incoming::Message {
                principal,
                text,
                images,
                parent,
                ..
            } => {
                self.directory.under(&self.tree.root().summary.id, parent);
                self.said(&principal, text, images).await;
            }
            Incoming::Click {
                question, choice, ..
            } => self.clicked(&question, &choice).await,
            Incoming::Action {
                principal,
                session,
                request_id,
                action,
                value,
                ..
            } => {
                self.pressed(&principal, session, &request_id, action, value)
                    .await
            }
        }
    }

    /// A reply that answers the open question is that answer; anything else
    /// is the next thing to work on.
    async fn said(&mut self, principal: &str, text: String, images: Vec<Image>) {
        match self.adapter.command(&text) {
            Some(ChannelCommand::NewSession) => return self.new_session().await,
            Some(ChannelCommand::Stop) => return self.stop().await,
            None => {}
        }
        match self.answering(&text) {
            Some((id, answer)) => self.settles(id, answer).await,
            None => {
                let intent = IntentId::mint();
                let input = Input::Text {
                    headline: None,
                    text,
                    images,
                    origin: incoming_origin(principal, &self.key),
                    delivery: Delivery::Wake,
                };
                self.handle.submit(intent.clone(), input.clone());
                self.deliverer.expecting_input(intent, input);
                self.acknowledge().await;
            }
        }
    }

    async fn stop(&mut self) {
        self.handle
            .interrupt(IntentId::mint(), bingo_sdk::InterruptScope::Head);
        if let Err(error) = self.post("任务已停止。").await {
            tracing::warn!(%error, key = %self.key, "the stop notice could not be sent");
        }
        self.ended(false).await;
    }

    async fn new_session(&mut self) {
        self.stop_current_turn().await;
        if let Err(error) = self
            .host
            .reconfigure(&self.tree.root().summary.id, SessionChange::Key(None))
            .await
        {
            tracing::warn!(%error, key = %self.key, "the old session could not release its route");
            if let Err(error) = self.post("新会话开启失败。").await {
                tracing::warn!(%error, key = %self.key, "the new-session failure notice could not be sent");
            }
            return;
        }
        self.retire_asked().await;
        let attachment = match fresh_session(self.host.clone(), self.key.clone(), self.cwd.clone())
            .await
        {
            Ok(attachment) => attachment,
            Err(error) => {
                tracing::warn!(%error, key = %self.key, "the new session could not open");
                if let Err(restore) = self
                    .host
                    .reconfigure(
                        &self.tree.root().summary.id,
                        SessionChange::Key(Some(self.key.clone())),
                    )
                    .await
                {
                    tracing::warn!(
                        %restore,
                        key = %self.key,
                        "the old session route could not be restored"
                    );
                }
                if let Err(error) = self.post("新会话开启失败。").await {
                    tracing::warn!(%error, key = %self.key, "the new-session failure notice could not be sent");
                }
                return;
            }
        };
        self.replace_attachment(attachment);
        if let Err(error) = self.post("新会话已开启。").await {
            tracing::warn!(%error, key = %self.key, "the new-session notice could not be sent");
        }
    }

    async fn stop_current_turn(&mut self) {
        let busy = self.tree.root().busy();
        if !busy && self.streaming.is_none() && self.working.is_none() {
            return;
        }
        self.handle
            .interrupt(IntentId::mint(), bingo_sdk::InterruptScope::Head);
        let ops = self.deliverer.finish_stream();
        self.perform(ops).await;
        if let Err(error) = self.post("任务已停止。").await {
            tracing::warn!(%error, key = %self.key, "the old-turn notice could not be sent");
        }
        self.ended(false).await;
    }

    async fn retire_asked(&mut self) {
        let ids: Vec<_> = self.asked.keys().cloned().collect();
        let outcome = withdrawn(&CancelReason::Superseded);
        for id in ids {
            if let Err(error) = self.settle(&id, &outcome).await {
                tracing::warn!(%error, key = %self.key, "the old question could not be retired");
            }
        }
    }

    fn replace_attachment(&mut self, attachment: Attachment) {
        let old_root = self.tree.root().summary.id.clone();
        let Attachment {
            session,
            snapshot,
            events,
            handle,
            ..
        } = attachment;
        let parent = self.directory.parent(&old_root);
        self.directory.leave(&old_root);
        self.directory.sit(
            session.clone(),
            Seat {
                adapter: Arc::clone(&self.adapter),
                conversation: self.conversation.clone(),
                parent,
            },
        );
        self.tree = TreeState::new(snapshot);
        self.events = events;
        self.handle = handle;
        self.deliverer.reset();
        self.streaming = None;
        self.working = None;
        self.asked.clear();
        self.drawn.clear();
    }

    fn answering(&self, text: &str) -> Option<(InteractionId, Answer)> {
        self.asked
            .values()
            .find_map(|asked| Some((asked.question.id.clone(), asked.question.parse(text)?)))
    }

    async fn clicked(&mut self, id: &InteractionId, choice: &str) {
        let Some(answer) = self.asked.get(id).and_then(|a| a.question.pick(choice)) else {
            return;
        };
        self.settles(id.clone(), answer).await;
    }

    /// A reply either answers the interaction or fills one slot of a form: a
    /// set asked one message at a time is answered once, at the last of them
    /// (M53). The message just answered says what it was answered with and
    /// loses its buttons, so no live button ever outlives its question.
    async fn settles(&mut self, id: InteractionId, answer: Answer) {
        let Some(asked) = self.asked.get(&id) else {
            return;
        };
        match asked.question.clone().settles(answer.clone()) {
            Settled::Whole(whole) => self.answer(id, whole),
            Settled::More(next) => {
                let outcome = asked.question.decided(&answer);
                if let Err(error) = self.settle(&id, &outcome).await {
                    tracing::warn!(%error, key = %self.key, "the answered question kept its buttons");
                }
                self.perform(vec![Op::Finalize {
                    text: String::new(),
                    question: Some(*next),
                }])
                .await;
            }
        }
    }

    /// Always `Pointer`, on both rungs. The kernel reads the activation for
    /// one thing: whether a keystroke could have landed on a prompt that had
    /// just appeared under it, which it guards against for 400 ms. Neither a
    /// button press nor a message composed and sent in a chat can be that
    /// accident, and an answer silently refused as `NOT_READY` would look to
    /// a person like a chat that had stopped listening.
    fn answer(&self, id: InteractionId, answer: Answer) {
        self.handle
            .answer(IntentId::mint(), id, answer, Activation::Pointer);
    }
}

async fn fresh_session(
    host: HostHandle,
    name: String,
    cwd: std::path::PathBuf,
) -> Result<Attachment, KernelError> {
    host.open(
        SessionSelector::Create {
            spec: SessionSpec {
                cwd,
                key: Some(name.clone()),
                ..SessionSpec::default()
            },
        },
        ClientIdentity {
            name,
            surface: SURFACE_ID.into(),
        },
        OpenOptions::with_children(),
    )
    .await
}

/// Post to the chat: under the message that started this where the platform
/// threads, and as a message of its own where it does not.
async fn post(
    adapter: Arc<dyn ChannelAdapter>,
    to: Conversation,
    parent: Option<Posted>,
    text: String,
    mode: Mode,
) -> Result<Posted, ChannelError> {
    match (adapter.threads(), &parent) {
        (Some(threads), Some(parent)) => threads.reply(&to, parent, &text, mode).await,
        _ => adapter.send(&to, &text, mode).await,
    }
}

/// Sleep until the coalescer's timer, or never when nothing is held.
async fn tick(due: Option<Instant>) {
    match due {
        Some(at) => tokio::time::sleep_until(tokio::time::Instant::from_std(at)).await,
        None => std::future::pending().await,
    }
}

/// Continue the session this chat already has, or start one under its key.
async fn attach(
    host: &HostHandle,
    key: &str,
    cwd: std::path::PathBuf,
) -> Result<Attachment, KernelError> {
    let who = ClientIdentity {
        name: key.to_string(),
        surface: SURFACE_ID.to_string(),
    };
    // The whole tree: a sub-agent's permission prompt reaches a person only
    // through the attachment that can see it (ADR-0010 §3). Its forwarder
    // heals a lag itself and back-pressures rather than dropping frames, so
    // no `Lagged` marker reaches this stream.
    let options = OpenOptions::with_children();
    match host
        .open(
            SessionSelector::ByKey {
                key: key.to_string(),
            },
            who.clone(),
            options,
        )
        .await
    {
        Err(error) if error.code == ErrorCode::SessionNotFound => {
            host.open(
                SessionSelector::Create {
                    spec: SessionSpec {
                        cwd,
                        key: Some(key.to_string()),
                        ..SessionSpec::default()
                    },
                },
                who,
                options,
            )
            .await
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{fixtures, loopback::Loopback};
    use bingo_sdk::{CloseReason, ParentLink, SessionId};

    #[tokio::test]
    async fn a_deleted_child_leaves_the_chat_fold() {
        let host = bingo_sdk::testing::TestHost::with_sessions(Vec::new()).handle();
        let adapter = Arc::new(Loopback::new(Default::default()));
        let mut runner = Runner::open(
            &host,
            adapter,
            Conversation::direct("chat"),
            "/tmp".into(),
            Gate::default(),
            Directory::default(),
            mpsc::channel(1).1,
        )
        .await
        .unwrap();
        let mut summary = fixtures::summary();
        summary.id = SessionId::from_raw("child");
        summary.parent = Some(ParentLink {
            session: runner.tree.root().summary.id.clone(),
            item: None,
        });
        let id = summary.id.clone();
        let announce = Frame {
            session: id.clone(),
            ..fixtures::frame(1, Event::SessionUpdated { summary })
        };
        runner.saw(&announce).await;
        assert!(runner.tree.get(&id).is_some());
        let deleted = Frame {
            session: id.clone(),
            ..fixtures::frame(
                2,
                Event::SessionClosed {
                    reason: CloseReason::Deleted,
                },
            )
        };
        runner.saw(&deleted).await;
        assert!(runner.tree.get(&id).is_none());
        runner
            .saw(&Frame {
                session: id.clone(),
                ..fixtures::says(3, "late", "gone")
            })
            .await;
        assert!(runner.tree.get(&id).is_none());
    }
}

fn incoming_origin(principal: &str, conversation: &str) -> Origin {
    Origin {
        render_surface: None,
        voice: Some(bingo_sdk::Voice::Said),
        surface: SURFACE_ID.into(),
        principal: Some(principal.into()),
        conversation: Some(conversation.into()),
    }
}

#[cfg(test)]
mod voice_tests {
    #[test]
    fn an_incoming_message_is_said_by_its_sender() {
        let origin = super::incoming_origin("sender", "conversation");
        assert_eq!(origin.voice, Some(bingo_sdk::Voice::Said));
        assert_eq!(origin.principal.as_deref(), Some("sender"));
        assert_eq!(origin.conversation.as_deref(), Some("conversation"));
    }
}

#[cfg(test)]
mod person_surface {
    /// An unstamped line from here is the person's only if the sdk says so.
    #[test]
    fn this_surface_is_one_a_person_types_through() {
        assert!(bingo_sdk::PERSON_SURFACES.contains(&super::SURFACE_ID));
    }
}
