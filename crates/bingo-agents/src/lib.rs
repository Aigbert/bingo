//! Sub-agents: an agent *is* a child session (ADR-0010). This plugin owns
//! every noun the kernel refuses to — agent, name, definition, team — and adds
//! no machinery of its own: a spawn is `open(Create)`, a message is `deliver`,
//! a roster is `sessions{parent}`, and `@name` is a submit hook that
//! redirects.
//!
//! Seven tools, two hooks, two commands:
//!
//! - `SpawnAgent` mints a child under the calling tool item and delivers the
//!   prompt. In the foreground it waits for the child's final text; in the
//!   background it returns the name and leaves a watcher to wake the parent.
//!   With `reopen` it hands the prompt to the child of that name instead.
//! - `SendMessage` wakes an agent — a child, a teammate beside the caller, or
//!   `parent` — or posts into a room's journal, `ListAgents` reads the
//!   tree, `ListModels` reads the model catalogue,
//!   `SetThinking` moves how hard this session or a child thinks.
//! - `StopAgent` ends a child's running turn and `DismissAgent` deletes an
//!   idle one: an agent ends where it began, by whoever started it
//!   (ADR-0060).
//! - `@name rest` in the composer reaches the child of that name.
//! - A root session opening in a project with a `.bingo/team.json` seats the
//!   roles it declares, as children of itself.
//! - `/agents` shows the roster a person needs, and `/agents stop|dismiss
//!   <name>` is the person's spelling of the two verbs; `/team` what was
//!   declared.
//! - `agents.team` is the one door onto `.bingo/team.json` for the plugins
//!   that own its other keys: this plugin parses the file, and nobody else
//!   knows where it is (ADR-0031).
//!
//! Every tool but one is declared read-only and trusted: none of them reads
//! or writes anything outside the process, and what a child then does is
//! gated in the child, against the child's own directory and rules. The one
//! is `DismissAgent`, which deletes a journal on disk and says so.

mod command;
mod definition;
mod dismiss;
pub mod guide;
mod hook;
mod layers;
mod library;
mod list;
mod message;
mod models;
mod names;

mod control;
mod note;
mod service;
mod spawn;
mod stop;
mod team;
mod thinking;
mod watch;

use std::sync::Arc;

use async_trait::async_trait;
use bingo_sdk::{
    Command, Contribution, Hook, Plugin, PluginError, PluginManifest, Registrar, Tool, ToolTraits,
};

use bingo_services::team as team_service;
pub use command::AgentsCommand;
pub use definition::Definition;
pub use dismiss::DismissAgentTool;
pub use hook::AtNameHook;
pub use list::ListAgentsTool;
pub use message::MessageTool;
pub use models::ListModelsTool;
pub use note::NOTE;
pub use spawn::SpawnAgentTool;
pub use stop::StopAgentTool;
pub use team::{SeatHook, TeamCommand, TeamFile};
pub use thinking::SetThinkingTool;

static MANIFEST: PluginManifest = PluginManifest {
    id: "bingo.agents",
    version: env!("CARGO_PKG_VERSION"),
    uses: &[bingo_services::room_post::KEY.name()],
    provides: &[
        "tool:SpawnAgent",
        "tool:SendMessage",
        "tool:ListAgents",
        "tool:ListModels",
        "tool:SetThinking",
        "tool:StopAgent",
        "tool:DismissAgent",
        "hook:agents",
        "hook:team",
        "hook:agents.reports",
        "command:agents",
        "command:team",
        team_service::CAPABILITY,
        "service:bingo.agents.agent",
        bingo_services::agents_control::CAPABILITY,
        "service:bingo.agents.pages",
    ],
    requires: &[],
    // Definitions are files, not settings, and the limits on a session tree
    // are the kernel's.
    config: None,
};

/// Both service faces refer to the same file owner.
fn team_file() -> Contribution {
    let owner: Arc<dyn team_service::TeamFile> = Arc::new(TeamFile);
    Contribution::service(
        &team_service::KEY,
        owner.clone(),
        Some(team_service::wire(owner)),
    )
}

/// What every tool here is but one. They read the session tree and post into
/// a queue: nothing outside the process changes, and a child's own calls are
/// gated in the child, so trusting these traits costs a person nothing.
/// None is concurrency-safe — a spawn and a message that raced would agree on
/// neither a name nor an order. `DismissAgent` starts from these and takes
/// back `read_only`: it is the one that deletes.
pub(crate) fn traits() -> ToolTraits {
    ToolTraits {
        read_only: true,
        trusted: true,
        concurrency_safe: false,
        ..ToolTraits::default()
    }
}

/// Registers the seven tools, the two hooks and the two commands. Nothing here
/// holds the host: a tool reads it from its call, a hook from its context.
#[derive(Debug, Default)]
pub struct AgentsPlugin {
    service: std::sync::OnceLock<Arc<service::AgentService>>,
    reports: Arc<report::Reports>,
}

#[async_trait]
impl Plugin for AgentsPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }

    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        let definitions = Arc::new(library::Definitions::default());
        let mut spawn = SpawnAgentTool::new(self.reports.clone());
        spawn.definitions = definitions.clone();
        registrar.tool(Arc::new(spawn) as Arc<dyn Tool>);
        let service = Arc::new(service::AgentService::new(
            registrar.env().clone(),
            self.reports.clone(),
            definitions,
        ));
        self.service
            .set(service.clone())
            .map_err(|_| PluginError::Failed("agents already registered".into()))?;
        registrar.tool(Arc::new(MessageTool) as Arc<dyn Tool>);
        registrar.tool(Arc::new(ListAgentsTool) as Arc<dyn Tool>);
        registrar.tool(Arc::new(ListModelsTool) as Arc<dyn Tool>);
        registrar.tool(Arc::new(SetThinkingTool) as Arc<dyn Tool>);
        registrar.tool(Arc::new(StopAgentTool) as Arc<dyn Tool>);
        registrar.tool(Arc::new(DismissAgentTool) as Arc<dyn Tool>);
        registrar.add(Contribution::Hook(Arc::new(AtNameHook) as Arc<dyn Hook>));
        registrar.add(Contribution::Hook(
            Arc::new(SeatHook::new(registrar.env().clone())) as Arc<dyn Hook>,
        ));
        registrar.add(Contribution::Command(
            Arc::new(AgentsCommand) as Arc<dyn Command>
        ));
        registrar.add(Contribution::Command(
            Arc::new(TeamCommand) as Arc<dyn Command>
        ));
        registrar.add(Contribution::Hook(Arc::new(report::ReportHook(
            self.reports.clone(),
        ))));
        registrar.add(team_file());
        registrar.add(Contribution::service(
            &service::KEY,
            service.clone(),
            Some(service.clone()),
        ));
        let control: Arc<dyn bingo_services::agents_control::AgentsControl> = service.clone();
        registrar.add(Contribution::service(
            &bingo_services::agents_control::KEY,
            control.clone(),
            Some(bingo_services::agents_control::wire(control)),
        ));
        registrar.add(guide::contribution(registrar));
        Ok(())
    }
    async fn start(&self, host: bingo_sdk::HostHandle) -> Result<(), PluginError> {
        if let Some(service) = self.service.get() {
            service.bind(&host);
        }
        Ok(())
    }
    async fn stop(&self) -> Result<(), PluginError> {
        self.reports.stop();
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests;

#[cfg(test)]
mod plugin_tests {
    use super::*;
    use bingo_sdk::Env;

    fn registrar() -> Registrar {
        Registrar::new(
            "bingo.agents",
            serde_json::Value::Null,
            Env::rooted("/tmp"),
            Default::default(),
        )
    }

    #[test]
    fn agent_noun_declares_spawn_list_and_register() {
        let mut registrar = registrar();
        AgentsPlugin::default().register(&mut registrar).unwrap();
        let face = registrar
            .into_contributions()
            .into_iter()
            .find_map(|contribution| {
                if let Contribution::Service {
                    wire: Some(wire), ..
                } = contribution
                {
                    return wire.face().filter(|face| face.noun == "agent");
                }
                None
            })
            .expect("agent noun");
        assert!(face.find("spawn").unwrap().unsafe_call);
        assert!(face.find("list").is_some());
        assert!(face.find("register").is_some());
    }

    #[test]
    fn registering_reads_nothing_and_contributes_what_the_manifest_promises() {
        let mut registrar = registrar();
        AgentsPlugin::default()
            .register(&mut registrar)
            .expect("register");
        let contributions = registrar.into_contributions();
        assert_eq!(contributions.len(), MANIFEST.provides.len());
        let tools: Vec<String> = contributions
            .iter()
            .filter_map(|c| match c {
                Contribution::Tool(tool) => Some(tool.spec().name),
                _ => None,
            })
            .collect();
        assert_eq!(
            tools,
            [
                "SpawnAgent",
                "SendMessage",
                "ListAgents",
                "ListModels",
                "SetThinking",
                "StopAgent",
                "DismissAgent",
            ]
        );
        let hooks: Vec<String> = contributions
            .iter()
            .filter_map(|c| match c {
                Contribution::Hook(hook) => Some(hook.id().to_string()),
                _ => None,
            })
            .collect();
        assert_eq!(
            hooks,
            ["agents.at-name", team_service::KEY.name(), "agents.reports"]
        );
        let commands: Vec<String> = contributions
            .iter()
            .filter_map(|c| match c {
                Contribution::Command(command) => Some(command.spec().name),
                _ => None,
            })
            .collect();
        assert_eq!(commands, ["agents", "team"]);
        let services: Vec<(&str, bool)> = contributions
            .iter()
            .filter_map(|c| match c {
                Contribution::Service { key, wire, .. } => Some((key.as_str(), wire.is_some())),
                _ => None,
            })
            .collect();
        let page = bingo_sdk::Pages::key(MANIFEST.id);
        assert_eq!(
            services,
            [
                (team_service::KEY.name(), true),
                (service::KEY.name(), true),
                (bingo_services::agents_control::KEY.name(), true),
                (page.name(), false)
            ],
            "the team file has both faces; the page stays in process"
        );
    }

    #[test]
    fn every_tool_is_read_only_trusted_and_alone_but_the_one_that_deletes() {
        let traits = traits();
        assert!(traits.read_only && traits.trusted);
        assert!(!traits.concurrency_safe && !traits.destructive && !traits.edit);
        let mut registrar = registrar();
        AgentsPlugin::default()
            .register(&mut registrar)
            .expect("register");
        for contribution in registrar.into_contributions() {
            let Contribution::Tool(tool) = contribution else {
                continue;
            };
            let traits = tool.traits(&serde_json::Value::Null);
            let deletes = tool.spec().name == "DismissAgent";
            assert_eq!(traits.destructive, deletes, "{}", tool.spec().name);
            assert_eq!(traits.read_only, !deletes, "{}", tool.spec().name);
            assert!(traits.trusted, "{}", tool.spec().name);
        }
    }
}

mod report;

#[cfg(test)]
mod control_tests;
