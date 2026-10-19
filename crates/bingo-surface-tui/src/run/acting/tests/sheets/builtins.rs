//! The kernel's own commands a sheet hands its choices to, answering as
//! bingo-core's built-ins do: a switch is a summary the session folds.
use super::kernel::Kernel;
use super::world::Switched;
use bingo_sdk::*;
use bingo_services::{agents_control, checkpoints, mcp};
use std::sync::Arc;

fn spec(name: &str, hint: &str) -> CommandSpec {
    CommandSpec {
        keys: vec![],
        hidden: false,
        opens: true,
        name: name.into(),
        aliases: vec![],
        hint: hint.into(),
        args: ArgSpec::None,
        instant: true,
        family: "session".into(),
    }
}

pub(super) fn register(kernel: &Kernel, switched: &Switched) {
    kernel.add(Arc::new(Model(kernel.clone())));
    kernel.add(Arc::new(Think(kernel.clone())));
    kernel.add(Arc::new(Rewind));
    kernel.add(Arc::new(Status));
    kernel.add(Arc::new(Plugins(kernel.clone(), switched.clone())));
    let kept: Arc<dyn checkpoints::Checkpoints> = Arc::new(Kept);
    serve(kernel, checkpoints::KEY.name(), kept);
    let crew: Arc<dyn agents_control::AgentsControl> = Arc::new(Crew(kernel.clone()));
    serve(kernel, agents_control::KEY.name(), crew);
    let servers: Arc<dyn mcp::Servers> = Arc::new(Servers);
    serve(kernel, mcp::KEY.name(), servers);
}

/// One in-process service under its key, as a plugin's registration adds it.
pub(super) fn serve<T: ?Sized + Send + Sync + 'static>(kernel: &Kernel, key: &str, value: Arc<T>) {
    let service = service::Service {
        value: Arc::new(value),
        wire: None,
    };
    kernel.host().services().add(key.into(), service).unwrap();
}

/// `/model provider/id` switches the session's model.
struct Model(Kernel);

#[async_trait::async_trait]
impl Command for Model {
    fn spec(&self) -> CommandSpec {
        spec("model", "model, thinking and providers")
    }
    async fn run(&self, args: &str, _: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let Some((provider, id)) = args
            .split_whitespace()
            .next()
            .and_then(|a| a.split_once('/'))
        else {
            return Ok(CommandOutcome::Applied {
                message: Some("Choose with /model <provider/model>.".into()),
            });
        };
        let mut summary = self.0.root().summary;
        summary.provider = Some(provider.into());
        summary.model = Some(id.into());
        self.0.broadcast(Event::SessionUpdated { summary });
        Ok(CommandOutcome::Applied {
            message: Some(format!("⇄ {id} · {provider}")),
        })
    }
}

/// `/think word` sets how hard the session thinks.
struct Think(Kernel);

#[async_trait::async_trait]
impl Command for Think {
    fn spec(&self) -> CommandSpec {
        spec("think", "how hard it thinks")
    }
    async fn run(&self, args: &str, _: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let word = args.split_whitespace().next().unwrap_or_default();
        let Some(effort) = Effort::spoken(word) else {
            return Err(KernelError::new(ErrorCode::InvalidInput, "not a level"));
        };
        let mut summary = self.0.root().summary;
        summary.effort = effort;
        self.0.broadcast(Event::SessionUpdated { summary });
        Ok(CommandOutcome::Applied {
            message: Some(format!("thinking: {}", Effort::word(effort))),
        })
    }
}

/// `/status` as the core lists it; a surface that shows sheets is answered
/// by the status owner before it runs.
struct Status;

#[async_trait::async_trait]
impl Command for Status {
    fn spec(&self) -> CommandSpec {
        spec("status", "session, context and cost")
    }
    async fn run(&self, _: &str, _: &CommandContext) -> Result<CommandOutcome, KernelError> {
        Err(KernelError::new(
            ErrorCode::InvalidInput,
            "a sheet scene draws no view",
        ))
    }
}

/// `/rewind turn` as bingo-checkpoints answers it: the files back, then the
/// conversation, then the toast that counts the files.
struct Rewind;

#[async_trait::async_trait]
impl Command for Rewind {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            opens: false,
            instant: false,
            ..spec(
                "rewind",
                "go back to a turn: its files and the conversation",
            )
        }
    }
    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let turn = TurnId::from_raw(args.split_whitespace().next().unwrap_or_default());
        let kept = checkpoints::find(&cx.host).unwrap();
        let touched = kept.touched(&cx.session, &turn).await.unwrap();
        let restored = touched.map_or(0, |t| t.files.len());
        cx.host.rewind(&cx.session, &turn).await?;
        Ok(CommandOutcome::Surface {
            action: toast::Say::ok(format!("rewound · {restored} files restored")).action(),
            fallback: None,
        })
    }
}

/// Every turn of the world changed the same two files and ran two commands.
struct Kept;

#[async_trait::async_trait]
impl checkpoints::Checkpoints for Kept {
    async fn touched(
        &self,
        _: &SessionId,
        _: &TurnId,
    ) -> Result<Option<checkpoints::Touched>, ServiceError> {
        let files = [
            "crates/bingo-core/src/session/queue.rs",
            "crates/bingo-core/tests/queue_steer.rs",
        ]
        .into_iter()
        .map(|path| checkpoints::File {
            path: path.into(),
            changed: true,
            fate: checkpoints::Fate::Restored,
        })
        .collect();
        Ok(Some(checkpoints::Touched {
            files,
            shell_calls: 2,
        }))
    }
}

/// The world's teammates, in the order they arrived.
struct Crew(Kernel);

#[async_trait::async_trait]
impl agents_control::AgentsControl for Crew {
    async fn roster(
        &self,
        _: agents_control::Session,
    ) -> Result<Vec<agents_control::AgentRow>, ServiceError> {
        let home = self.0.root().summary;
        let roster = crate::test_support::ember::world::roster(&home);
        Ok(roster
            .into_iter()
            .skip(1)
            .map(|summary| agents_control::AgentRow { summary })
            .collect())
    }
    async fn team(
        &self,
        _: agents_control::Session,
    ) -> Result<Vec<agents_control::TeamRow>, ServiceError> {
        Ok(vec![])
    }
    async fn stop(&self, _: agents_control::Named) -> Result<String, ServiceError> {
        Err(ServiceError("no agent runs in a sheet scene".into()))
    }
    async fn dismiss(&self, _: agents_control::Named) -> Result<String, ServiceError> {
        Err(ServiceError("no agent runs in a sheet scene".into()))
    }
    async fn message(&self, _: agents_control::Message) -> Result<String, ServiceError> {
        Err(ServiceError("no agent runs in a sheet scene".into()))
    }
    async fn reopen(&self, _: agents_control::Message) -> Result<String, ServiceError> {
        Err(ServiceError("no agent runs in a sheet scene".into()))
    }
}

/// `/plugins enable|disable id` as the core's plugin switch answers it: the
/// switch is kept, and the plugins catalogue is said to have changed.
struct Plugins(Kernel, Switched);

#[async_trait::async_trait]
impl Command for Plugins {
    fn spec(&self) -> CommandSpec {
        spec("plugins", "plugins and MCP servers")
    }
    async fn run(&self, args: &str, _: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let mut words = args.split_whitespace();
        let (Some(verb @ ("enable" | "disable")), Some(id)) = (words.next(), words.next()) else {
            return Ok(CommandOutcome::Applied { message: None });
        };
        self.1.flip(id);
        self.0.announce(GatewayEvent::CatalogChanged {
            kind: CatalogKind::Plugins,
        });
        Ok(CommandOutcome::Applied {
            message: Some(format!("{id}: {verb}d from the next turn")),
        })
    }
}

/// The world's MCP servers.
struct Servers;

#[async_trait::async_trait]
impl mcp::Servers for Servers {
    async fn rows(&self) -> Vec<mcp::ServerRow> {
        super::world::servers()
    }
}
