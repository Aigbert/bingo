//! Context: what the model is told about the project, and what happens when
//! the conversation outgrows the window.
//!
//! The kernel owns the ruler — the thresholds, the acceptance rule and the
//! breaker — and this plugin owns the strategy: what a summary says, which
//! files reach the prompt, and where the model keeps what it remembers
//! (ADR-0006, ADR-0044, ADR-0049).

mod baseline;
mod branch;
mod compact;
mod estimate;
mod files;
pub mod guide;
mod instructions;
mod memory;
mod prompt;
mod root;
mod split;
mod stream;
mod tail;

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod git;
#[cfg(test)]
mod query;
#[cfg(test)]
mod scripted;

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use bingo_sdk::{
    Command, ContextContributor, Contribution, Plugin, PluginError, PluginManifest, Registrar,
};

pub use compact::SummaryCompactor;
pub use instructions::InstructionsContributor;
pub use memory::{MemoryCommand, MemoryContributor};

pub(crate) static MANIFEST: PluginManifest = PluginManifest {
    id: "bingo.context",
    version: env!("CARGO_PKG_VERSION"),
    uses: &[],
    provides: &[
        "compactor:summary",
        "context:instructions",
        "context:memory",
        "command:memory",
        "service:bingo.context.pages",
        bingo_services::memory::CAPABILITY,
    ],
    requires: &[],
    config: None,
};

/// Registers the summary compactor, the instruction files, the two memory
/// scopes, their reading service and the command that lists or forgets them.
/// The model remains their only author (ADR-0049, ADR-0092).
#[derive(Debug, Default, Clone, Copy)]
pub struct ContextPlugin;

#[async_trait]
impl Plugin for ContextPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }

    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        let config_dir = registrar.env().config_dir.clone();
        let data_dir: PathBuf = registrar.env().data_dir.clone();
        registrar.add(Contribution::Compactor(Arc::new(SummaryCompactor)));
        registrar.add(Contribution::Context(
            Arc::new(InstructionsContributor::new(config_dir.clone()))
                as Arc<dyn ContextContributor>,
        ));
        registrar.add(Contribution::Context(
            Arc::new(MemoryContributor::new(data_dir.clone())) as Arc<dyn ContextContributor>,
        ));
        registrar.add(Contribution::Command(Arc::new(MemoryCommand::new(
            data_dir.clone(),
            config_dir.clone(),
        )) as Arc<dyn Command>));
        registrar.add(Contribution::Hook(Arc::new(baseline::BaselineHook::new(
            data_dir.clone(),
        ))));
        registrar.add(guide::contribution(registrar));
        registrar.add(Contribution::Hook(Arc::new(memory::row::MemoryRows)));
        registrar.add(memory::service::contribution(data_dir, config_dir));
        registrar.add(Contribution::Hook(Arc::new(branch::Branch::default())));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::Env;
    use serde_json::json;

    fn contributions(config: serde_json::Value) -> Vec<Contribution> {
        let mut registrar = Registrar::new(
            "bingo.context",
            config,
            Env::rooted("/tmp/home"),
            Default::default(),
        );
        ContextPlugin.register(&mut registrar).expect("register");
        registrar.into_contributions()
    }

    /// Baselines and transcript rows retain their original contribution order.
    #[test]
    fn the_plugin_registers_a_compactor_two_contributors_the_command_and_two_hooks() {
        let contributions = contributions(json!({}));
        // The hook is nobody's promise: it is the baseline's own, and the
        // manifest names the five things another plugin could ask for.
        assert_eq!(contributions.len(), MANIFEST.provides.len() + 3);
        assert!(matches!(contributions[0], Contribution::Compactor(_)));
        assert!(matches!(contributions[1], Contribution::Context(_)));
        assert!(matches!(contributions[2], Contribution::Context(_)));
        assert!(matches!(contributions[3], Contribution::Command(_)));
        let Contribution::Hook(hook) = &contributions[4] else {
            panic!("the fifth contribution is the baseline hook");
        };
        assert_eq!(hook.id(), "context:baselines");
        assert_eq!(hook.matcher().points, [bingo_sdk::HookPoint::Session]);
        match &contributions[5] {
            Contribution::Service { key, wire, .. } => {
                assert_eq!(key, bingo_sdk::Pages::key(MANIFEST.id).name());
                assert!(wire.is_none(), "a page is read in process");
            }
            other => panic!("expected the page service, got {other:?}"),
        }
    }
}
