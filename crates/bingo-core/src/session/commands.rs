//! Command dispatch (ADR-0008): a line is parsed here, its command looked up
//! in the one table, and run on its own task; the actor turns the outcome
//! into an ack. Nothing here awaits a command.

use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};

use bingo_sdk::*;
use futures::FutureExt;
use tokio_util::task::TaskTracker;

use super::mailbox::{Mailbox, Msg};

/// What the session actor needs from the host to run commands.
pub struct Services {
    pub commands: Vec<Arc<dyn Command>>,
    /// Commands that arrive after I/O, consulted when the table has no such name.
    pub command_sources: Vec<Arc<dyn CommandSource>>,
    /// Weak: the host holds every session's mailbox, and a session must not
    /// hold the host.
    pub host: Weak<dyn HostApi>,
    /// Who drew what, so a key's answer can be vouched for.
    pub drawings: super::Drawings,
}

impl Services {
    /// No commands and no host: a session that only takes prose.
    pub fn none() -> Self {
        Self {
            commands: Vec::new(),
            command_sources: Vec::new(),
            host: Weak::<crate::host::Unavailable>::new(),
            drawings: super::Drawings::default(),
        }
    }
}

/// The surface a command's own prompt carries (ADR-0008 §3). A `Prompt` is
/// the command speaking, not the keyboard: what re-enters the session is a
/// skill's body, and the surface is how every client tells the two apart.
pub(super) const SURFACE: &str = "command";

/// The one name that is its own prefix.
const BANG: &str = bingo_sdk::command::SHELL;

/// Whether a name is the shell line's (ADR-0008 §5).
pub(crate) fn is_shell(name: &str) -> bool {
    name == BANG
}

/// Shell execution requires the same person origin as command consent.
/// Actions never run a shell line.
pub(super) fn may_run_shell(input: &Input) -> bool {
    match input {
        Input::Text { origin, .. } => crate::context::the_persons_own(origin),
        Input::Action { .. } => false,
    }
}

/// A typed line is the person's ask when their own surface signed it. A press
/// is their ask only to the handler of the drawing they pressed: any other
/// command a control names is the drawer's word, not the person's, so a
/// drawn button cannot stand in for a typed person-only verb (fail closed).
pub(super) fn asked(input: &Input) -> bool {
    invocation::current().is_empty()
        && match input {
            Input::Text { origin, .. } => crate::context::the_persons_own(origin),
            Input::Action { action, origin } => {
                bingo_sdk::CommandSpec::is_ui(&action.name)
                    && origin.as_ref().is_some_and(crate::context::the_persons_own)
            }
        }
}

/// What a name nothing answers to is refused with. A `!` that found nothing
/// is not a misspelling: the shell a bang line runs in is a plugin's, and a
/// host that registered none has no shell at all rather than a typo.
pub(super) fn unknown(name: &str) -> String {
    match name {
        BANG => format!("no shell here: nothing answers to {}", spelled(BANG)),
        name => format!("unknown command: {}", spelled(name)),
    }
}

/// A name and its argument text, as the actor parses them (ADR-0008 §1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Parsed {
    pub name: String,
    pub args: String,
}

impl Parsed {
    /// The line as it would have been typed: what a person recognises the run
    /// by, and — for an `Input::Action`, which was never typed at all — what
    /// they would have typed for it.
    pub(super) fn typed(&self) -> String {
        let name = spelled(&self.name);
        match (self.args.is_empty(), self.name.as_str()) {
            (true, _) => name,
            (false, BANG) => format!("{name}{}", self.args),
            (false, _) => format!("{name} {}", self.args),
        }
    }
}

/// How a command's name is written: `!` is its own prefix, everything else
/// follows a slash.
pub(super) fn spelled(name: &str) -> String {
    match name {
        BANG => BANG.to_string(),
        name => format!("/{name}"),
    }
}

/// `/name rest` → `name`, `!rest` → `!`, an action → its name; prose → `None`.
pub(super) fn parse(input: &Input) -> Option<Parsed> {
    match input {
        Input::Text { text, origin, .. } if crate::context::person_surface(origin) => {
            parse_line(text)
        }
        Input::Text { .. } => None,
        Input::Action { action, .. } => Some(Parsed {
            name: action.name.clone(),
            args: match &action.args {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Null => String::new(),
                other => other.to_string(),
            },
        }),
    }
}

fn parse_line(text: &str) -> Option<Parsed> {
    let (name, args) = bingo_sdk::command::command_line(text)?;
    Some(Parsed {
        name: name.to_string(),
        args: args.to_string(),
    })
}

/// How a command was invoked, kept for as long as it runs: the line that
/// asked for it, and the origin whatever it produces re-enters the session
/// with.
#[derive(Clone, Debug)]
pub(super) struct Invocation {
    pub surface: Option<bingo_sdk::render::SurfaceName>,
    pub line: String,
    pub origin: Origin,
}

impl Invocation {
    /// What the submitter's origin becomes once a command stands between them
    /// and the session: the same person, in the same conversation, through the
    /// command surface (ADR-0008 §3). An action's signature says only whose
    /// press it was (`asked`), so its invocation starts from no origin.
    pub(super) fn of(input: &Input, parsed: &Parsed) -> Self {
        let submitted = match input {
            Input::Text { origin, .. } => origin.clone(),
            Input::Action { .. } => Origin::default(),
        };
        Self {
            surface: match input {
                Input::Text { origin, .. } => origin.render_surface,
                Input::Action { origin, .. } => {
                    origin.as_ref().and_then(|origin| origin.render_surface)
                }
            },
            line: bingo_sdk::command::recorded_line(&parsed.typed()).into(),
            origin: Origin {
                voice: Some(bingo_sdk::Voice::Quiet),
                surface: SURFACE.into(),
                ..submitted
            },
        }
    }

    /// A command's prompt as the journal keeps it: the line that asked for it,
    /// then what it produced. The line is written down once, here — a client
    /// reads the run back off the item rather than off a field beside it.
    pub(super) fn prompt(&self, text: &str) -> String {
        format!("{}\n\n{text}", self.line)
    }
}

/// One run the actor asks for.
pub(super) struct Run {
    pub intent: IntentId,
    pub invocation: Invocation,
    pub command: Arc<dyn Command>,
    pub args: String,
    /// A non-instant run, which the queue waits behind.
    pub holds: bool,
    pub asked: bool,
    /// The built-in whose own drawing handed out the key this run answers.
    pub vouched: Option<String>,
}

/// A run that is over, as the actor turns it into an ack.
pub(super) struct Finished {
    pub invocation: Invocation,
    /// Whether it was the run holding the queue.
    pub held: bool,
    /// The built-in vouching for the answer: the drawing was its own, and
    /// so is the outcome, untouched by any link.
    pub vouched: Option<String>,
}

/// The table and the runs in flight.
#[derive(Clone)]
pub(super) struct Lookup {
    table: Vec<Arc<dyn Command>>,
    sources: Vec<Arc<dyn CommandSource>>,
    cwd: PathBuf,
}

struct InFlight {
    invocation: Invocation,
    cancel: CancellationToken,
    vouched: Option<String>,
    /// Set by the run once its outcome proves to be the command's own.
    own: Arc<AtomicBool>,
}

pub(super) struct Commands {
    lookup: Lookup,
    host: Weak<dyn HostApi>,
    drawings: super::Drawings,
    mailbox: Mailbox,
    session: SessionId,
    tracker: TaskTracker,
    cancel: CancellationToken,
    inflight: HashMap<IntentId, InFlight>,
    /// The non-instant command whose run holds the queue behind it.
    holding: Option<IntentId>,
}

impl Commands {
    pub(super) fn new(
        services: Services,
        mailbox: Mailbox,
        cwd: PathBuf,
        tracker: TaskTracker,
        cancel: CancellationToken,
    ) -> Self {
        Self {
            lookup: Lookup {
                table: services.commands,
                sources: services.command_sources,
                cwd,
            },
            tracker,
            cancel,
            host: services.host,
            drawings: services.drawings,
            session: mailbox.id().clone(),
            mailbox,
            inflight: HashMap::new(),
            holding: None,
        }
    }

    pub(super) fn lookup(&self) -> Lookup {
        self.lookup.clone()
    }

    /// The built-in that drew the key handler an action presses, if one did.
    pub(super) fn vouched(&self, input: &Input) -> Option<String> {
        match input {
            Input::Action { action, .. } => self.drawings.vouches(action),
            Input::Text { .. } => None,
        }
    }

    pub(super) fn interrupt(&self) -> Option<IntentId> {
        let intent = self.holding.as_ref()?;
        self.inflight.get(intent)?.cancel.cancel();
        Some(intent.clone())
    }

    /// A non-instant command is running: the queue waits.
    pub(super) fn busy(&self) -> bool {
        self.holding.is_some()
    }

    /// Run on a task that reports back by mail.
    pub(super) fn spawn(
        &mut self,
        run: Run,
        hooks: crate::turn::HookSet,
        hook_cx: HookContext,
    ) -> Result<(), KernelError> {
        let host = self
            .host
            .upgrade()
            .ok_or_else(|| KernelError::new(ErrorCode::SessionClosed, "the host is gone"))?;
        let cancel = self.cancel.child_token();
        let cx = CommandContext {
            cancel: cancel.clone(),
            session: self.session.clone(),
            cwd: self.lookup.cwd.clone(),
            host: HostHandle(host),
        };
        let Run {
            intent,
            invocation,
            command,
            args,
            holds,
            asked,
            vouched,
        } = run;
        let pressed = vouched.is_some();
        let own = Arc::new(AtomicBool::new(false));
        let input = bingo_sdk::catalogue::CommandRun {
            name: command.spec().name,
            args,
            surface: invocation.surface,
        };
        self.inflight.insert(
            intent.clone(),
            InFlight {
                invocation,
                cancel: cancel.clone(),
                vouched,
                own: own.clone(),
            },
        );
        if holds {
            self.holding = Some(intent.clone());
        }
        let mailbox = self.mailbox.clone();
        self.tracker.spawn(bingo_sdk::invocation::carry(async move {
            let report = crate::chain::Report::default();
            let work =
                crate::chain::command::run(command, input, &cx, &hooks, &hook_cx, report.clone());
            let work = bingo_sdk::render::under_press(pressed, work);
            let future =
                AssertUnwindSafe(bingo_sdk::render::under_input(asked, work)).catch_unwind();
            let returned = super::running::supervised(future, &cancel).await;
            let outcome = settled(returned, &cancel, &own);
            for notice in report.take() {
                let _ = mailbox.record(notice).await;
            }
            mailbox.send(Msg::CommandFinished { intent, outcome });
        }));
        Ok(())
    }

    /// The run is over; how it was invoked comes back for a `Prompt` outcome.
    pub(super) fn finish(&mut self, intent: &IntentId) -> Option<Finished> {
        let held = self.holding.as_ref() == Some(intent);
        if held {
            self.holding = None;
        }
        self.inflight.remove(intent).map(|run| Finished {
            invocation: run.invocation,
            held,
            vouched: run.vouched.filter(|_| run.own.load(Ordering::Acquire)),
        })
    }
}

/// A supervised run as the outcome its ack reports; whether the outcome is
/// the command's own goes to `own`.
fn settled(
    returned: Option<std::thread::Result<Result<crate::chain::command::Ran, KernelError>>>,
    cancel: &CancellationToken,
    own: &AtomicBool,
) -> Result<CommandOutcome, KernelError> {
    match returned {
        _ if cancel.is_cancelled() => Err(interrupted()),
        Some(Ok(ran)) => ran.map(|ran| {
            own.store(ran.own, Ordering::Release);
            ran.outcome
        }),
        Some(Err(panic)) => Err(KernelError::new(
            ErrorCode::Internal,
            format!("the command panicked: {}", super::panic_message(panic)),
        )),
        None => Err(interrupted()),
    }
}

impl Lookup {
    /// The table first, then each source in order (ADR-0009).
    pub(super) async fn find(&self, name: &str) -> Option<Arc<dyn Command>> {
        if let Some(found) = named(&self.table, name) {
            return Some(found);
        }
        for source in &self.sources {
            if let Some(found) = named(&source.commands(&self.cwd).await, name) {
                return Some(found);
            }
        }
        None
    }
}

pub(super) fn interrupted() -> KernelError {
    KernelError::new(ErrorCode::Interrupted, "stopped by the person")
}

fn named(table: &[Arc<dyn Command>], name: &str) -> Option<Arc<dyn Command>> {
    table
        .iter()
        .find(|c| {
            let spec = c.spec();
            spec.name == name || spec.aliases.iter().any(|a| a == name)
        })
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn text(t: &str) -> Input {
        Input::text(t, Origin::surface("rpc"))
    }

    #[test]
    fn a_slash_line_is_a_name_and_its_arguments() {
        assert_eq!(
            parse(&text("/model  anthropic/x ")),
            Some(Parsed {
                name: "model".into(),
                args: "anthropic/x".into()
            })
        );
        assert_eq!(
            parse(&text("  /help")),
            Some(Parsed {
                name: "help".into(),
                args: String::new()
            })
        );
        assert_eq!(parse(&text("/")), None, "a bare slash is prose");
        assert_eq!(parse(&text("hello /world")), None);
    }

    #[test]
    fn a_bang_line_is_the_shell_command_with_the_rest_verbatim() {
        assert_eq!(
            parse(&text("!ls -la | head")),
            Some(Parsed {
                name: "!".into(),
                args: "ls -la | head".into()
            })
        );
    }

    /// What a run is recognised by, whatever door it came through.
    #[test]
    fn a_run_is_labelled_by_the_line_that_would_have_typed_it() {
        let typed = |line: &str| parse(&text(line)).map(|p| p.typed());
        assert_eq!(
            typed("/model  anthropic/x "),
            Some("/model anthropic/x".into())
        );
        assert_eq!(typed("  /help"), Some("/help".into()));
        assert_eq!(typed("!ls -la | head"), Some("!ls -la | head".into()));
        assert_eq!(typed("!"), Some("!".into()));
        let action = Input::Action {
            action: Action {
                name: "permission".into(),
                args: json!("plan"),
            },
            origin: None,
        };
        assert_eq!(
            parse(&action).map(|p| p.typed()),
            Some("/permission plan".into()),
            "a button nobody typed still reads as the line it stands for"
        );
    }

    /// The submitter stays who they are; only the surface says a command now
    /// stands between them and the session.
    #[test]
    fn an_invocation_keeps_the_submitter_and_takes_the_command_surface() {
        let input = Input::text(
            "/guide do the thing",
            Origin {
                render_surface: None,
                voice: None,
                surface: "channels".into(),
                principal: Some("ou_person".into()),
                conversation: Some("loopback/oc_1".into()),
            },
        );
        let parsed = parse(&input).expect("a correspondent can use an ordinary command");
        assert!(
            !asked(&input),
            "a correspondent cannot grant person consent"
        );
        assert!(!may_run_shell(&input), "a correspondent cannot run a shell");
        let invocation = Invocation::of(&input, &parsed);
        assert_eq!(invocation.line, "/guide do the thing");
        assert_eq!(
            invocation.origin,
            Origin {
                render_surface: None,
                voice: Some(bingo_sdk::Voice::Quiet),
                surface: SURFACE.into(),
                principal: Some("ou_person".into()),
                conversation: Some("loopback/oc_1".into()),
            }
        );
        assert_eq!(
            invocation.prompt("Read the guide."),
            "/guide do the thing\n\nRead the guide.",
            "the typed line is the item's first line and its only record"
        );
    }

    #[test]
    fn an_action_keeps_render_metadata_without_person_authority() {
        let input = Input::action(
            Action {
                name: "model".into(),
                args: serde_json::Value::String(String::new()),
            },
            Origin::surface("plugin:picker").on(bingo_sdk::render::SurfaceName::Terminal),
        );
        let invocation = Invocation::of(&input, &parse(&input).unwrap());
        assert_eq!(
            invocation.surface,
            Some(bingo_sdk::render::SurfaceName::Terminal)
        );
        assert!(!asked(&input));
        assert!(!may_run_shell(&input));
        assert_eq!(invocation.origin.principal, None);
    }

    /// The bareness the fold reads a person's own line by is the same bareness
    /// a shell line is allowed by: a door that signs what it sends may not
    /// run one (M65).
    #[test]
    fn only_the_persons_own_line_may_run_a_shell_command() {
        let from = |origin: Origin| {
            may_run_shell(&Input::Text {
                text: "!ls".into(),
                images: Vec::new(),
                origin,
                delivery: Delivery::Wake,
            })
        };
        assert!(from(Origin::surface("tui")));
        assert!(from(Origin::surface("print")));
        assert!(!from(Origin {
            render_surface: None,
            voice: None,
            surface: "agent".into(),
            principal: Some("scout".into()),
            conversation: None,
        }));
        assert!(!from(Origin {
            render_surface: None,
            voice: None,
            surface: "channels".into(),
            principal: Some("ou_person".into()),
            conversation: Some("loopback/oc_1".into()),
        }));
        assert!(
            !from(Origin {
                render_surface: None,
                voice: None,
                surface: "peer".into(),
                principal: None,
                conversation: Some("#design".into()),
            }),
            "a post signs where it came from"
        );
        assert!(
            !from(Origin::surface("kernel")),
            "the kernel's own voice is not the person's"
        );
        assert!(
            !may_run_shell(&Input::Action {
                action: Action {
                    name: BANG.into(),
                    args: json!("ls"),
                },
                origin: Some(Origin::surface("tui")),
            }),
            "a button runs no shell line, even the person's own"
        );
    }

    /// An action is the person's only when a person surface signed it: one a
    /// plugin fired, one from a surface nobody presses, and one signed by no
    /// surface at all are nobody's (fail closed).
    #[test]
    fn an_action_is_asked_only_when_a_person_surface_signed_it() {
        let fired = |origin: Option<Origin>| {
            asked(&Input::Action {
                action: Action {
                    name: "plugins:ui".into(),
                    args: json!({"handler":"h","event":"press"}),
                },
                origin,
            })
        };
        for surface in PERSON_SURFACES {
            assert!(fired(Some(Origin::surface(surface))), "{surface}");
        }
        assert!(!fired(Some(Origin::surface("plugin:counter"))));
        assert!(!fired(Some(Origin::surface("plugin-kit"))));
        assert!(!fired(Some(Origin::surface("kernel"))));
        assert!(!fired(None), "an unsigned action is nobody's");
    }

    /// A drawn button naming a person-only verb is the drawer's word even
    /// when the person pressed it: only a typed line asks for the verb.
    #[test]
    fn a_pressed_button_is_never_a_typed_command() {
        for surface in PERSON_SURFACES {
            for (name, args) in [
                ("plugins", "trust evil full"),
                ("plugins", "approve p full"),
                ("permission", "wider"),
            ] {
                let pressed = Input::action(
                    Action {
                        name: name.into(),
                        args: json!(args),
                    },
                    Origin::surface(surface),
                );
                assert!(!asked(&pressed), "{surface}: {name} {args}");
                let typed = Input::text(format!("/{name} {args}"), Origin::surface(surface));
                assert!(
                    asked(&typed),
                    "{surface}: the typed line stays the person's"
                );
            }
        }
    }

    #[test]
    fn generated_origins_are_never_the_persons_commands() {
        for surface in ["wake", "plugin:probe", "scheduler", "hook:probe", "unknown"] {
            for voice in [
                None,
                Some(Voice::Said),
                Some(Voice::Note),
                Some(Voice::Quiet),
            ] {
                let mut origin = Origin::surface(surface);
                origin.voice = voice;
                let input = Input::text("!touch marker", origin.clone());
                assert!(!may_run_shell(&input), "{origin:?}");
                assert!(!asked(&input), "{origin:?}");
                assert!(parse(&input).is_none(), "{origin:?}");
                let slash = Input::text("/plugins trust probe full", origin);
                assert!(parse(&slash).is_none());
            }
        }
    }

    #[test]
    fn a_person_surface_cannot_turn_a_note_or_signature_into_consent() {
        for surface in PERSON_SURFACES {
            for voice in [Voice::Note, Voice::Quiet] {
                let origin = Origin::surface(surface).voiced(voice);
                assert!(!asked(&Input::text(
                    "/plugins trust probe full",
                    origin.clone()
                )));
                assert!(!asked(&Input::action(
                    Action {
                        name: "plugins".into(),
                        args: json!("trust probe full")
                    },
                    origin
                )));
            }
            let mut origin = Origin::surface(surface);
            origin.principal = Some("someone else".into());
            assert!(!asked(&Input::action(
                Action {
                    name: "plugins".into(),
                    args: json!("trust probe full")
                },
                origin
            )));
        }
    }

    /// A `!` nothing answers to is not a misspelling.
    #[test]
    fn a_shell_line_with_no_shell_says_that_is_what_is_missing() {
        assert!(is_shell(BANG) && !is_shell("model"));
        assert!(unknown(BANG).contains("shell"), "{}", unknown(BANG));
        assert_eq!(unknown("nope"), "unknown command: /nope");
    }

    #[test]
    fn an_action_is_its_name_and_its_arguments_as_text_or_json() {
        let action = |args| Input::Action {
            action: Action {
                name: "permission".into(),
                args,
            },
            origin: None,
        };
        assert_eq!(
            parse(&action(json!("plan"))).map(|p| p.args),
            Some("plan".into())
        );
        assert_eq!(
            parse(&action(json!(null))).map(|p| p.args),
            Some(String::new())
        );
        assert_eq!(
            parse(&action(json!({"mode": "plan"}))).map(|p| p.args),
            Some(r#"{"mode":"plan"}"#.into())
        );
    }
}
