//! One configured adapter, as an `Agent`.
//!
//! A turn is one `session/prompt`, held open. Only the new user message
//! crosses: an ACP session is stateful and holds everything before it
//! (ADR-0035 §3), so replaying the folded context would tell the agent its own
//! history back. The system prompt does not cross either — the agent has its
//! own — nor `max_tokens` (ADR-0035 §6). `Effort` and the model do, but not
//! on this wire: they are the agent's own knobs, turned between turns through
//! the options it declared (ADR-0037, `crate::knobs`). The tools do, but not
//! on this wire: they were handed over at `session/new` as an MCP server the
//! agent dials, and the request's list of them only says what that server now
//! offers (ADR-0036 §1).

use std::sync::Arc;

use agent_client_protocol_schema::v1::{
    CancelNotification, ContentBlock, ImageContent, PromptRequest, PromptResponse,
    SessionId as AcpSessionId, TextContent,
};
use async_trait::async_trait;
use bingo_sdk::{
    Agent, AgentEvent, AgentStream, AgentTurn, CancellationToken, ContentPart, Effort, Judge,
    ModelInfo, ProviderError, SessionId, ToolSpec,
};
use futures::stream;
use tokio::sync::mpsc;

use crate::config::{AGENT, Adapter};
use crate::error::AcpError;
use crate::events::Mapper;
use crate::knobs::Wanted;
use crate::session::{Link, Sessions};

type Yielded = Result<AgentEvent, ProviderError>;

pub struct AcpAgent {
    name: String,
    adapter: Adapter,
    sessions: Arc<Sessions>,
}

impl AcpAgent {
    pub fn new(name: String, adapter: Adapter, sessions: Arc<Sessions>) -> Self {
        Self {
            name,
            adapter,
            sessions,
        }
    }

    /// The request, taken apart into what it takes to ask it — and taken
    /// apart by moving: the turn keeps every piece for as long as it might
    /// have to ask again, and a copy of the fold would be the conversation
    /// twice in memory.
    fn asking(&self, turn: AgentTurn) -> Asking {
        Asking {
            sessions: self.sessions.clone(),
            name: self.name.clone(),
            adapter: self.adapter.clone(),
            session: turn.session,
            asked: turn.input,
            tools: turn.tools,
            reasoning: turn.reasoning,
            model: turn.model,
            judge: turn.judge,
        }
    }
}

/// One request's question, and everything it takes to put it again.
///
/// A child that died between turns is replaced rather than asked (ADR-0035
/// §3). `Sessions::prepare` finds most of those deaths before the prompt is
/// written, but that check is an optimisation and not the rule: a child can go
/// in the moment between it and the write, and the rule has to hold wherever
/// the death is discovered. So the turn keeps what it was built from, and a
/// prompt that comes back as a dead pipe having said nothing is asked once
/// more on a new child.
struct Asking {
    sessions: Arc<Sessions>,
    name: String,
    adapter: Adapter,
    session: SessionId,
    judge: Arc<dyn Judge>,

    /// What the person just said, which is all of the fold that crosses.
    asked: Vec<ContentPart>,
    tools: Vec<ToolSpec>,
    reasoning: Option<Effort>,
    model: String,
}

impl Asking {
    /// A link ready to carry this request: the ladder climbed if it has to be,
    /// the doors handed this request's offer, the knobs turned to what it asks
    /// for. All of it, both times — a turn asked again is not a lesser turn.
    async fn ready(&self) -> Result<Arc<Link>, ProviderError> {
        let link = self
            .sessions
            .prepare_turn(&self.name, &self.adapter, &self.session)
            .await?;
        // The offer is this request's own tool list (ADR-0036 §1). A request
        // that moves it is what `tools/list_changed` is made of, so the bridge
        // hears about it before the prompt that will be answered with it.
        if link.observe(&self.tools).await {
            self.sessions.offer_changed().await;
        }
        // ADR-0037 §4: between turns, never inside one. What the request asks
        // for is applied to the agent before the prompt that will be answered
        // under it — and only what moved crosses.
        self.sessions
            .tune(
                &self.name,
                &link,
                Wanted {
                    effort: self.reasoning,
                    model: &self.model,
                },
            )
            .await;
        Ok(link)
    }

    /// The same, on a child of its own. The one that was there is let go of
    /// first — and the person told it went — so the ladder opens a
    /// conversation instead of handing back the dead one.
    async fn afresh(&self) -> Result<Arc<Link>, ProviderError> {
        self.sessions.bury(&self.name, &self.session).await;
        self.ready().await
    }

    /// What crosses: what the link has to say first, then what the person
    /// said. Taken from the link that will carry it, never before — a freshly
    /// opened child has its own first words, and a turn asked again on one
    /// must say them.
    async fn prompt(&self, link: &Link) -> PromptRequest {
        let mut prompt = said(&self.asked, link.capabilities.prompt_capabilities.image);
        if let Some(preamble) = link.take_preamble().await {
            match prompt.first_mut() {
                Some(ContentBlock::Text(first)) => {
                    first.text = format!("{preamble}\n\n{}", first.text)
                }
                _ => prompt.insert(0, ContentBlock::Text(TextContent::new(preamble))),
            }
        }
        PromptRequest::new(AcpSessionId::new(link.acp.as_str()), prompt)
    }
}

#[async_trait]
impl Agent for AcpAgent {
    fn id(&self) -> &str {
        &self.name
    }
    async fn run(&self, turn: AgentTurn) -> Result<AgentStream, ProviderError> {
        let asking = self.asking(turn);
        let link = asking.ready().await?;
        Ok(hold(link, asking, CancellationToken::new()).await)
    }
    async fn models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(catalogue(
            self.sessions.models(&self.name, &self.adapter).await,
        ))
    }
}

/// `agent` first and always: it is the one id an ACP instance serves whatever
/// the agent has said, and it means "whatever you would have used". An agent
/// that happens to call one of its own models that too is served once.
fn catalogue(theirs: Vec<ModelInfo>) -> Vec<ModelInfo> {
    std::iter::once(ModelInfo {
        id: AGENT.to_string(),
        display: None,
    })
    .chain(theirs.into_iter().filter(|model| model.id != AGENT))
    .collect()
}

/// Keep the user's order and decide from the link carrying this attempt.
fn said(parts: &[ContentPart], images: bool) -> Vec<ContentBlock> {
    let prompt: Vec<_> = parts
        .iter()
        .filter_map(|part| match part {
            ContentPart::Text { text } => Some(ContentBlock::Text(TextContent::new(text.clone()))),
            ContentPart::Image(image) if images => Some(ContentBlock::Image(ImageContent::new(
                image.data.to_string(),
                image.media_type.clone(),
            ))),
            ContentPart::Image(image) => Some(ContentBlock::Text(TextContent::new(format!(
                "[picture not sent: {} {}; this agent does not take pictures]",
                image.media_type,
                bingo_sdk::bytes::words(image.decoded_len()),
            )))),
            _ => None,
        })
        .collect();
    if prompt.is_empty() {
        vec![ContentBlock::Text(TextContent::new(""))]
    } else {
        prompt
    }
}

/// An agent keeps its state when it answers cancel promptly. Ten seconds
/// allows cleanup but bounds a wedged child after the kernel drops the turn.
pub const CANCEL_GRACE: std::time::Duration = std::time::Duration::from_secs(10);

/// The turn, as a stream. The asking runs on a task of its own so the stream
/// is there to be read the moment it is handed over.
async fn hold(link: Arc<Link>, asking: Asking, cancel: CancellationToken) -> AgentStream {
    let (out, events) = mpsc::unbounded_channel();
    let owned = link.clone();
    link.spawn(async move {
        round(owned.clone(), asking, cancel, Telling::new(out)).await;
        owned.clear_judge().await;
    });
    Box::pin(stream::unfold(events, |mut events| async move {
        events.recv().await.map(|event| (event, events))
    }))
}

/// The turn's own stream, and whether anything has gone down it. One fact,
/// because it is the fact that says whether the question may be put again: a
/// turn that has said nothing can be asked afresh, and one that has said
/// something cannot, because the second telling would be the first one twice.
struct Telling {
    out: mpsc::UnboundedSender<Yielded>,
    said: bool,
}

impl Telling {
    fn new(out: mpsc::UnboundedSender<Yielded>) -> Self {
        Self { out, said: false }
    }

    /// `false` once nobody is reading: the turn was dropped and there is no
    /// reason to keep folding.
    fn tell(&mut self, events: Vec<AgentEvent>) -> bool {
        self.said |= !events.is_empty();
        events
            .into_iter()
            .all(|event| self.out.send(Ok(event)).is_ok())
    }

    fn failed(&self, error: ProviderError) {
        let _ = self.out.send(Err(error));
    }
}

/// How one `session/prompt` ended.
enum Attempt {
    /// Answered, refused, cancelled or dropped: whatever there was to say has
    /// been said, and the turn is over.
    Done,
    /// The pipe died; whether it had said anything determines whether it can retry.
    Lost(AcpError),
}

/// The turn, and — once, and only if the first attempt said nothing at all —
/// the same turn on a new child (ADR-0035 §3).
///
/// The death that ended the first attempt is not what the person is told: the
/// notice that a child went is said where it is buried, and the turn's own
/// answer is whatever the second child makes of the question.
async fn round(link: Arc<Link>, asking: Asking, cancel: CancellationToken, mut out: Telling) {
    let Attempt::Lost(gone) = attempt(&link, &asking, &cancel, &mut out).await else {
        return;
    };
    if out.said {
        fail_lost(&asking, &out, gone).await;
        return;
    }
    match asking.afresh().await {
        Ok(fresh) => {
            if let Attempt::Lost(gone) = attempt(&fresh, &asking, &cancel, &mut out).await {
                fail_lost(&asking, &out, gone).await;
            }
        }
        Err(refused) => out.failed(refused),
    }
}

async fn fail_lost(asking: &Asking, out: &Telling, gone: AcpError) {
    asking.sessions.bury(&asking.name, &asking.session).await;
    let error = if out.said {
        ProviderError::Transport {
            message: format!("agent stopped mid-answer: {gone}"),
        }
    } else {
        gone.into()
    };
    out.failed(error);
}

/// One `session/prompt`, and what the conversation keeps of it: the agent's
/// last word on the context it holds, which outlives the turn that heard it,
/// and the window it named, which outlives the session.
async fn attempt(
    link: &Arc<Link>,
    asking: &Asking,
    cancel: &CancellationToken,
    out: &mut Telling,
) -> Attempt {
    // The conversation's last reading, not the turn's: what the agent holds
    // fell or grew since somebody last looked, and either way this turn is
    // the one that sees it (ADR-0055 §4).
    link.judge(asking.judge.clone(), out.out.clone(), cancel.clone())
        .await;
    let mut mapper = Mapper::holding(link.held().await);
    let attempt = fold(link, asking, cancel, out, &mut mapper).await;
    link.clear_judge().await;
    // A turn the agent said nothing about leaves what it last said standing.
    if let Some(reading) = mapper.held() {
        link.hold(reading).await;
    }
    attempt
}

/// One `session/prompt`, held open: updates folded as they arrive, an
/// interrupt sent as `session/cancel`, and the finish written when the prompt
/// answers.
async fn fold(
    link: &Arc<Link>,
    asking: &Asking,
    cancel: &CancellationToken,
    out: &mut Telling,
    mapper: &mut Mapper,
) -> Attempt {
    let mut updates = link.listen().await;
    let prompt = asking.prompt(link).await;
    let waiting = link.connection.call(prompt);
    tokio::pin!(waiting);
    let mut told = false;
    let grace = tokio::time::sleep(CANCEL_GRACE);
    tokio::pin!(grace);
    let answered = loop {
        tokio::select! {
            biased;
            () = &mut grace, if told => {
                link.expire();
                out.failed(ProviderError::Timeout);
                return Attempt::Done;
            }
            answer = &mut waiting => break answer,
            () = stopped(cancel, &out.out), if !told => {
                cancel.cancel();
                told = true;
                grace.as_mut().reset(tokio::time::Instant::now() + CANCEL_GRACE);
                // The link owns this wait even after M116 drops the turn.
                let _ = link
                    .connection
                    .notify(CancelNotification::new(AcpSessionId::new(link.acp.as_str())));
            }
            Some(note) = updates.recv() => {
                // Keep draining until the cancel answer or grace, even when
                // the turn has dropped its receiver.
                out.tell(mapper.update(note.update));
            }
        }
    };
    // Whatever the agent said between its last update and its answer.
    while let Ok(note) = updates.try_recv() {
        if !out.tell(mapper.update(note.update)) {
            return Attempt::Done;
        }
    }
    settle(answered, mapper, cancel, out)
}

async fn stopped(cancel: &CancellationToken, out: &mpsc::UnboundedSender<Yielded>) {
    tokio::select! {
        () = cancel.cancelled() => {},
        () = out.closed() => {},
    }
}

/// What the prompt answered with, and what it leaves the turn. A dead pipe
/// with nothing said and no interrupt to explain it is the one outcome worth a
/// second child; the agent's own refusal, a shape this build cannot read, and
/// a death after the first word are all this turn's answer.
fn settle(
    answered: Result<PromptResponse, AcpError>,
    mapper: &mut Mapper,
    cancel: &CancellationToken,
    out: &mut Telling,
) -> Attempt {
    match answered {
        Ok(response) => {
            out.tell(mapper.finish(&response));
            Attempt::Done
        }
        Err(gone) if transport(&gone) && !cancel.is_cancelled() => Attempt::Lost(gone),
        Err(failed) => {
            out.failed(failed.into());
            Attempt::Done
        }
    }
}

/// A failure of the pipe rather than of the agent: the child is gone, or has
/// stopped speaking. Anything the agent itself answered — a refusal in its own
/// words, an answer this build could not read — happened, and asking again
/// would only be asking it twice.
fn transport(error: &AcpError) -> bool {
    matches!(error, AcpError::Transport(_))
}

/// Every configured adapter is an agent with its own conversation.
pub fn agents(rows: Vec<(String, Adapter)>, sessions: &Arc<Sessions>) -> Vec<Arc<dyn Agent>> {
    rows.into_iter()
        .map(|(name, adapter)| {
            Arc::new(AcpAgent::new(name, adapter, sessions.clone())) as Arc<dyn Agent>
        })
        .collect()
}
