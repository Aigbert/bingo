//! The owners' commands and links, dispatched the way bingo-core dispatches them:
//! parse, look up, run the command chain, and answer with the core-shaped ack.
use bingo_sdk::*;
use futures::{channel::mpsc, future::BoxFuture};
use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

/// One live object every task of the scene shares: the owners, the root
/// session's state, and the events the surface has not been handed yet.
#[derive(Clone)]
pub(super) struct Kernel(Arc<Inner>);

struct Inner {
    host: testing::TestHost,
    commands: Mutex<Vec<Arc<dyn Command>>>,
    links: Mutex<Vec<Arc<dyn Interceptor>>>,
    observers: Mutex<Vec<Arc<dyn Hook>>>,
    root: Mutex<SessionState>,
    out: Mutex<Vec<Event>>,
    gateway: Mutex<Vec<GatewayEvent>>,
    prompts: Mutex<Vec<(IntentId, Input)>>,
    ports: Mutex<Vec<mpsc::UnboundedSender<Frame>>>,
    running: AtomicUsize,
}

impl Kernel {
    pub(super) fn new(host: testing::TestHost, root: SessionState) -> Self {
        Self(Arc::new(Inner {
            host,
            commands: Mutex::default(),
            links: Mutex::default(),
            observers: Mutex::default(),
            root: Mutex::new(root),
            out: Mutex::default(),
            gateway: Mutex::default(),
            prompts: Mutex::default(),
            ports: Mutex::default(),
            running: AtomicUsize::new(0),
        }))
    }

    pub(super) fn host(&self) -> &testing::TestHost {
        &self.0.host
    }

    /// Registers one owner as the host would: its commands, links and services.
    pub(super) fn register(&self, plugin: &dyn Plugin, env: &Env) {
        let mut registrar = Registrar::new(
            plugin.manifest().id,
            json!({}),
            env.clone(),
            Default::default(),
        );
        plugin.register(&mut registrar).unwrap();
        for contribution in registrar.into_contributions() {
            match contribution {
                Contribution::Command(command) => self.add(command),
                Contribution::Interceptor { link, .. } => self.0.links.lock().unwrap().push(link),
                Contribution::Hook(hook) if hook.matcher().points.contains(&HookPoint::Event) => {
                    self.0.observers.lock().unwrap().push(hook)
                }
                Contribution::Service { key, value, wire } => {
                    let service = service::Service { value, wire };
                    self.0.host.services().add(key, service).unwrap();
                }
                _ => {}
            }
        }
    }

    /// A kernel command, dispatched beside the owners'.
    pub(super) fn add(&self, command: Arc<dyn Command>) {
        self.0.commands.lock().unwrap().push(command);
    }

    /// The commands as bingo-core lists them: one entry per visible spec.
    pub(super) fn catalogue(&self) -> Vec<CatalogEntry> {
        let commands = self.0.commands.lock().unwrap().clone();
        commands
            .iter()
            .map(|command| command.spec())
            .filter(|spec| !spec.hidden)
            .map(|spec| CatalogEntry {
                id: spec.name.clone(),
                label: spec.hint.clone(),
                meta: serde_json::to_value(spec).unwrap(),
            })
            .collect()
    }

    pub(super) fn links(&self) -> Vec<Arc<dyn Interceptor>> {
        self.0.links.lock().unwrap().clone()
    }

    pub(super) fn root(&self) -> SessionState {
        self.0.root.lock().unwrap().clone()
    }

    pub(super) fn idle(&self) -> bool {
        self.0.running.load(Ordering::SeqCst) == 0
    }

    pub(super) fn take(&self) -> Vec<Event> {
        std::mem::take(&mut *self.0.out.lock().unwrap())
    }

    /// The prompts submitted since the provider last looked.
    pub(super) fn prompted(&self) -> Vec<(IntentId, Input)> {
        std::mem::take(&mut *self.0.prompts.lock().unwrap())
    }

    /// What the host says to every client at once, beside any session.
    pub(super) fn announce(&self, event: GatewayEvent) {
        self.0.gateway.lock().unwrap().push(event);
    }

    pub(super) fn announced(&self) -> Vec<GatewayEvent> {
        std::mem::take(&mut *self.0.gateway.lock().unwrap())
    }

    /// The root folds the event first, as the session actor journals before it
    /// fans out; every attachment then hears it, and the surface after them.
    pub(super) fn broadcast(&self, mut event: Event) {
        if let Event::ConfigChanged { config } = &mut event {
            config.kernel["render"] = serde_json::to_value(drawn(&self.links())).unwrap();
        }
        let frame = {
            let mut root = self.0.root.lock().unwrap();
            let frame = Frame {
                cause: None,
                session: root.summary.id.clone(),
                seq: Seq(root.seq.0 + 1),
                ts: root.summary.updated_at,
                event: event.clone(),
            };
            root.apply(&frame);
            frame
        };
        self.0
            .ports
            .lock()
            .unwrap()
            .retain(|port| port.unbounded_send(frame.clone()).is_ok());
        self.0.out.lock().unwrap().push(event);
        self.observe(frame);
    }

    /// Every event hook hears the frame on its own task, after the fan-out,
    /// as the host's observers do.
    fn observe(&self, frame: Frame) {
        let observers = self.0.observers.lock().unwrap().clone();
        for hook in observers {
            self.0.running.fetch_add(1, Ordering::SeqCst);
            let (kernel, frame) = (self.clone(), frame.clone());
            tokio::spawn(async move {
                hook.on_event(&frame, &kernel.hook_cx()).await;
                kernel.0.running.fetch_sub(1, Ordering::SeqCst);
            });
        }
    }

    /// A second attachment to the root session, as `host.open` gives a plugin.
    pub(super) fn attach(&self) -> Attachment {
        let (tx, rx) = mpsc::unbounded();
        self.0.ports.lock().unwrap().push(tx);
        let snapshot = self.root();
        Attachment {
            session: snapshot.summary.id.clone(),
            snapshot,
            history: None,
            events: Box::pin(rx),
            handle: SessionHandle(Arc::new(Port(self.clone()))),
        }
    }

    /// Runs one submission on its own task, as the session actor does; a
    /// prompt waits for the provider the scene plays.
    pub(super) fn submit(&self, intent: IntentId, input: Input) {
        if parse(&input).is_none() {
            self.0.prompts.lock().unwrap().push((intent, input));
            return;
        }
        self.0.running.fetch_add(1, Ordering::SeqCst);
        let kernel = self.clone();
        tokio::spawn(async move {
            let outcome = kernel.dispatch(&input).await;
            kernel.broadcast(Event::IntentAck { intent, outcome });
            kernel.0.running.fetch_sub(1, Ordering::SeqCst);
        });
    }

    async fn dispatch(&self, input: &Input) -> IntentOutcome {
        let Some((name, args)) = parse(input) else {
            return rejected("a prompt is the provider's");
        };
        let Some(command) = self.find(&name) else {
            return rejected(&format!("unknown command: /{name}"));
        };
        let (asked, pressed) = standing(input);
        let surface = match input {
            Input::Text { origin, .. } => origin.render_surface,
            Input::Action { origin, .. } => origin.as_ref().and_then(|o| o.render_surface),
        };
        let outcome = render::under_input(
            asked,
            render::under_press(pressed, self.run(command, args, surface)),
        )
        .await;
        match outcome {
            Ok(CommandOutcome::Record { body }) => self.recorded(*body),
            outcome => finished(outcome, drawer(input).filter(|_| pressed)),
        }
    }

    /// A receipt is an item in the root's transcript, and its words on the ack.
    fn recorded(&self, body: ItemBody) -> IntentOutcome {
        let message = body.receipt().map(str::to_owned);
        let item = self.record(body);
        let mut result = json!({ "item": item });
        if let Some(message) = message {
            result["message"] = json!(message);
        }
        IntentOutcome::Applied { result }
    }

    /// Takes the root back to just before `to_turn`, as the session actor
    /// does: the receipt first, then every item from the turn's first on.
    pub(super) fn rewind(&self, to_turn: TurnId) -> Result<u32, KernelError> {
        let root = self.root();
        let items = root.transcript.items();
        let Some(opened) = items
            .iter()
            .position(|item| item.turn.as_ref() == Some(&to_turn))
        else {
            return Err(KernelError::new(ErrorCode::InvalidInput, "no such turn"));
        };
        let dropped: Vec<ItemId> = items[opened..].iter().map(|item| item.id.clone()).collect();
        let count = dropped.len() as u32;
        self.record(ItemBody::Rewind {
            to_turn: to_turn.clone(),
            dropped: count,
        });
        self.broadcast(Event::Rewound {
            generation: self.root().transcript.generation() + 1,
            to_turn,
            dropped,
            files_restored: Vec::new(),
        });
        Ok(count)
    }

    /// One completed item in the root's transcript.
    fn record(&self, body: ItemBody) -> ItemId {
        let now = self.root().summary.updated_at;
        let item = Item {
            id: ItemId::mint(),
            turn: None,
            round: 0,
            status: ItemStatus::Completed,
            started_at: now,
            completed_at: Some(now),
            intent: None,
            body,
            meta: Default::default(),
        };
        let id = item.id.clone();
        self.broadcast(Event::ItemCompleted { item });
        id
    }

    fn find(&self, name: &str) -> Option<Arc<dyn Command>> {
        self.0
            .commands
            .lock()
            .unwrap()
            .iter()
            .find(|c| {
                let spec = c.spec();
                spec.name == name || spec.aliases.iter().any(|a| a == name)
            })
            .cloned()
    }

    async fn run(
        &self,
        command: Arc<dyn Command>,
        args: String,
        surface: Option<render::SurfaceName>,
    ) -> Result<CommandOutcome, KernelError> {
        let input = catalogue::CommandRun {
            name: command.spec().name,
            args,
            surface,
        };
        let value = serde_json::to_value(&input).unwrap();
        let entry = catalogue::COMMAND_RUN.clone();
        let links = self.matching(&entry, &value);
        let cx = self.hook_cx();
        let command_cx = CommandContext {
            surface,
            session: cx.session.clone(),
            cwd: cx.cwd.clone(),
            host: cx.host.clone(),
            cancel: Default::default(),
        };
        let last = Next::new(|value: Value| {
            Box::pin(async move {
                let input: catalogue::CommandRun = serde_json::from_value(value).unwrap();
                let outcome = command.run(&input.args, &command_cx).await?;
                Ok(
                    serde_json::to_value(catalogue::CommandReply::Outcome(Box::new(outcome)))
                        .unwrap(),
                )
            })
        });
        let reply = chain(&links, &entry, &cx, value, last).await?;
        let reply: catalogue::CommandReply = serde_json::from_value(reply).unwrap();
        Ok(reply.into())
    }

    /// One drawing, through every render link that matches it.
    pub(super) async fn render(&self, request: render::RenderRequest) -> Result<View, KernelError> {
        let value = serde_json::to_value(&request).unwrap();
        let entry = catalogue::UI_RENDER.clone();
        let links = self.matching(&entry, &value);
        let cx = self.hook_cx();
        let last = Next::new(|_| {
            Box::pin(async { Err(KernelError::new(ErrorCode::NotFound, "no owner drew it")) })
        });
        let drawn = chain(&links, &entry, &cx, value, last).await?;
        serde_json::from_value(drawn)
            .map_err(|e| KernelError::new(ErrorCode::Internal, e.to_string()))
    }

    fn matching(&self, entry: &catalogue::Entry, value: &Value) -> Vec<Arc<dyn Interceptor>> {
        self.links()
            .into_iter()
            .filter(|link| link.matcher().matches(entry, value))
            .collect()
    }

    fn hook_cx(&self) -> HookContext {
        let root = self.root();
        HookContext {
            item: None,
            session: root.summary.id.clone(),
            turn: None,
            cwd: root.summary.cwd.clone().into(),
            provider: None,
            model: None,
            host: self.0.host.handle(),
        }
    }
}

/// The drawings the owners' links answer, as bingo-core's config names them
/// so a surface asks only for what an owner draws.
fn drawn(links: &[Arc<dyn Interceptor>]) -> render::Matched {
    let mut matched = render::Matched::default();
    for link in links {
        let matcher = link.matcher();
        if !matcher.patterns.is_empty() && !matcher.patterns.iter().any(|p| p.matches("ui.render"))
        {
            continue;
        }
        let partials = matcher.render.unwrap_or_else(|| vec![matcher.prefilter]);
        for component in render::Component::ALL {
            let event = json!({ "component": component });
            for partial in &partials {
                if partial.is_null() || catalogue::prefilter(partial, &event) {
                    matched
                        .0
                        .entry(component)
                        .or_default()
                        .push(partial.clone());
                }
            }
        }
    }
    matched
}

/// Whether the person asked, and whether a press reached a drawing. The core
/// vouches a press against the drawing it handed out; every press in a scene
/// is on a drawing an owner just drew, so a handler's name stands for that.
fn standing(input: &Input) -> (bool, bool) {
    match input {
        Input::Text { origin, .. } => (persons(origin), false),
        Input::Action { action, origin } => {
            let ui = CommandSpec::is_ui(&action.name);
            (ui && origin.as_ref().is_some_and(persons), ui)
        }
    }
}

fn persons(origin: &Origin) -> bool {
    PERSON_SURFACES.contains(&origin.surface.as_str()) && effective_voice(origin) == Voice::Said
}

/// The links wrap the command outermost first, each handing on to the rest.
fn chain<'a>(
    links: &'a [Arc<dyn Interceptor>],
    entry: &'a catalogue::Entry,
    cx: &'a HookContext,
    input: Value,
    last: Next<'a, Value, Value>,
) -> BoxFuture<'a, Result<Value, KernelError>> {
    match links.split_first() {
        None => Box::pin(last.run(input)),
        Some((link, rest)) => Box::pin(link.chain(
            entry,
            cx,
            input,
            Next::new(move |value| chain(rest, entry, cx, value, last)),
        )),
    }
}

fn parse(input: &Input) -> Option<(String, String)> {
    match input {
        Input::Text { text, .. } => {
            let (name, args) = command::command_line(text)?;
            Some((name.into(), args.into()))
        }
        Input::Action { action, .. } => Some((
            action.name.clone(),
            match &action.args {
                Value::String(s) => s.clone(),
                Value::Null => String::new(),
                other => other.to_string(),
            },
        )),
    }
}

fn rejected(message: &str) -> IntentOutcome {
    IntentOutcome::Rejected {
        error: KernelError::new(ErrorCode::InvalidInput, message),
    }
}

/// The owner whose drawing a press reached: the one whose handler it names.
fn drawer(input: &Input) -> Option<String> {
    let Input::Action { action, .. } = input else {
        return None;
    };
    action.name.strip_suffix(":ui").map(str::to_owned)
}

/// The ack bingo-core's `command_finished` gives each outcome; only the
/// kernel says which owner drew the key a surface action answers.
fn finished(
    outcome: Result<CommandOutcome, KernelError>,
    vouched: Option<String>,
) -> IntentOutcome {
    let result = match outcome {
        Err(error) => return IntentOutcome::Rejected { error },
        Ok(CommandOutcome::Acknowledged { result }) => result,
        Ok(CommandOutcome::Surface { action, fallback }) => {
            let mut result = json!({"handled": true, "surface": action});
            if let Some(message) = fallback {
                result["message"] = json!(message);
            }
            if let Some(plugin) = vouched {
                result["drawnBy"] = json!(plugin);
            }
            result
        }
        Ok(CommandOutcome::Answer { answer }) => json!({"handled": true, "answer": answer}),
        Ok(CommandOutcome::Handled { handled }) => json!({"handled": handled}),
        Ok(CommandOutcome::Applied { message }) => message
            .map(|m| json!({"message": m}))
            .unwrap_or_else(|| json!({})),
        Ok(CommandOutcome::View { view }) => json!({"view": view}),
        Ok(CommandOutcome::Record { .. }) => {
            return rejected("a receipt is recorded before it is acknowledged");
        }
        Ok(CommandOutcome::Prompt { .. }) => {
            return rejected("a sheet scene runs no prompts");
        }
    };
    IntentOutcome::Applied { result }
}

/// A plugin's own attachment: what it submits runs through the same kernel.
struct Port(Kernel);

#[async_trait::async_trait]
impl SessionPort for Port {
    fn submit(&self, intent: IntentId, input: Input) {
        self.0.submit(intent, input);
    }
    fn interrupt(&self, _: IntentId, _: InterruptScope) {}
    fn answer(&self, _: IntentId, _: InteractionId, _: Answer, _: Activation) {}
    async fn history(&self, _: HistoryPage) -> Result<HistoryChunk, KernelError> {
        Err(KernelError::new(
            ErrorCode::InvalidInput,
            "no history in a sheet scene",
        ))
    }
    async fn events_since(&self, _: Seq) -> Result<FrameStream, KernelError> {
        Err(KernelError::new(
            ErrorCode::InvalidInput,
            "no replay in a sheet scene",
        ))
    }
}
