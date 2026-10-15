//! Checkpoints (ADR-0045): the bytes of a file before the turn that changed
//! them, and `/rewind` to put both the files and the conversation back.
//!
//! The pieces, in the order they depend on each other:
//!
//! - [`store`] — the directory: one turn per directory, one index line per
//!   file, the bytes beside it.
//! - [`hook`] — the map from a tool to the field naming the file it writes,
//!   and the `BeforeTool` hook that keeps it.
//! - [`restore`] — what going back to a turn does to the files, planned
//!   before a byte moves.
//! - [`command`] — `/rewind`, which is the whole of what a person touches.
//!
//! What is *not* here: anything a shell line wrote. A `Bash` call names no
//! path this could read, so its changes are not snapshotted and a rewind does
//! not claim to undo them (ADR-0045 §2).

mod after;
pub mod command;
pub mod guide;
pub mod hook;
mod preview;
pub mod restore;
pub mod store;

use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use bingo_sdk::{
    Command, Contribution, Hook, HostHandle, Plugin, PluginError, PluginManifest, Registrar,
    SessionFilter,
};

pub use command::RewindCommand;
pub use hook::SnapshotHook;
pub use store::Checkpoints;

static MANIFEST: PluginManifest = PluginManifest {
    id: "bingo.checkpoints",
    version: env!("CARGO_PKG_VERSION"),
    uses: &[],
    provides: &[
        "hook:checkpoints",
        "command:rewind",
        "service:bingo.checkpoints.pages",
        bingo_services::checkpoints::CAPABILITY,
    ],
    requires: &[],
    // Where the snapshots live follows the data directory; how many a turn
    // keeps follows what the turn edited. There is nothing to configure.
    config: None,
};

/// Registers the snapshot hook and `/rewind`, and collects the checkpoints of
/// sessions that are no longer there when the process starts.
#[derive(Default)]
pub struct CheckpointsPlugin {
    /// Built in `register`, where the environment is; used by `start`, which
    /// is handed nothing but the host.
    store: OnceLock<Arc<Checkpoints>>,
    host: Arc<OnceLock<HostHandle>>,
}

#[async_trait]
impl Plugin for CheckpointsPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }

    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        let store = Arc::new(Checkpoints::new(&registrar.env().data_dir));
        registrar.add(Contribution::Hook(
            Arc::new(SnapshotHook::new(store.clone())) as Arc<dyn Hook>,
        ));
        registrar.add(Contribution::Command(
            Arc::new(RewindCommand::new(store.clone())) as Arc<dyn Command>,
        ));
        let service: Arc<dyn bingo_services::checkpoints::Checkpoints> =
            Arc::new(preview::Preview {
                store: store.clone(),
                host: self.host.clone(),
            });
        registrar.add(Contribution::service(
            &bingo_services::checkpoints::KEY,
            service.clone(),
            Some(bingo_services::checkpoints::wire(service)),
        ));
        registrar.add(guide::contribution(registrar));
        self.store
            .set(store)
            .map_err(|_| PluginError::Failed("the checkpoints plugin registered twice".into()))
    }

    /// Nothing expires a checkpoint but the end of the session it belongs to
    /// (ADR-0045 §4). What the host lists is the whole of what exists — an
    /// empty listing is an answer, not a failure to answer, and a host with
    /// no store has no session anything could have rewound. Only an error is
    /// "cannot tell", and then nothing is collected.
    async fn start(&self, host: HostHandle) -> Result<(), PluginError> {
        let Some(store) = self.store.get() else {
            return Ok(());
        };
        let _ = self.host.set(host.clone());
        let alive = match host.sessions(SessionFilter::default()).await {
            Ok(sessions) => sessions,
            // Collection that cannot run is not a reason to refuse to start.
            Err(error) => {
                tracing::warn!(%error, "checkpoints were not collected");
                return Ok(());
            }
        };
        let kept: Vec<String> = alive.iter().map(|s| s.id.as_str().to_string()).collect();
        let gone = store.collect(&kept);
        if !gone.is_empty() {
            tracing::info!(count = gone.len(), "collected checkpoints of gone sessions");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod plugin_tests {
    use super::*;
    use bingo_sdk::{Env, SessionId, SessionSummary, TurnId};

    fn registered(home: &std::path::Path) -> Vec<Contribution> {
        let mut registrar = Registrar::new(
            MANIFEST.id,
            serde_json::Value::Null,
            Env::rooted(home),
            Default::default(),
        );
        CheckpointsPlugin::default()
            .register(&mut registrar)
            .expect("registering does no i/o");
        registrar.into_contributions()
    }

    #[test]
    fn the_manifest_says_what_it_provides_and_claims_no_settings() {
        assert_eq!(MANIFEST.id, "bingo.checkpoints");
        assert_eq!(
            MANIFEST.provides,
            [
                "hook:checkpoints",
                "command:rewind",
                &format!("service:{}", bingo_sdk::Pages::key(MANIFEST.id).name()),
                bingo_services::checkpoints::CAPABILITY,
            ]
        );
        assert!(MANIFEST.requires.is_empty());
        assert!(MANIFEST.config.is_none());
    }

    #[test]
    fn registering_contributes_a_hook_a_command_and_a_page_and_creates_nothing() {
        let home = tempfile::tempdir().expect("a temp home");
        let contributions = registered(home.path());
        assert_eq!(contributions.len(), MANIFEST.provides.len());
        assert!(matches!(contributions[0], Contribution::Hook(_)));
        assert!(matches!(contributions[1], Contribution::Command(_)));
        match &contributions[3] {
            Contribution::Service { key, wire, .. } => {
                assert_eq!(key, bingo_sdk::Pages::key(MANIFEST.id).name());
                assert!(wire.is_none(), "a page is read in process");
            }
            other => panic!("expected the page service, got {other:?}"),
        }
        assert!(
            !home.path().join(".bingo/data/checkpoints").exists(),
            "registering creates no directory"
        );
    }

    /// One host registers a plugin once.
    #[test]
    fn registering_twice_is_refused_rather_than_silently_forgotten() {
        let home = tempfile::tempdir().expect("a temp home");
        let plugin = CheckpointsPlugin::default();
        let mut first = Registrar::new(
            MANIFEST.id,
            serde_json::Value::Null,
            Env::rooted(home.path()),
            Default::default(),
        );
        plugin.register(&mut first).expect("the first");
        let mut again = Registrar::new(
            MANIFEST.id,
            serde_json::Value::Null,
            Env::rooted(home.path()),
            Default::default(),
        );
        assert!(plugin.register(&mut again).is_err());
    }

    /// A host holding one session, for the sweep to measure the store against.
    fn one_session(session: Option<SessionId>) -> HostHandle {
        let host = bingo_sdk::testing::TestHost::default();
        host.on_sessions(move |_| {
            let summaries = session
                .iter()
                .map(|id| SessionSummary {
                    id: id.clone(),
                    ..crate::tests::summary()
                })
                .collect();
            async move { Ok(summaries) }
        });
        host.on_open(|_, _, _| async { unreachable!("the sweep opens nothing") });
        host.on_close(|_, _| async { unreachable!("the sweep closes nothing") });
        host.on_delete(|_| async { unreachable!("the sweep deletes no session") });
        host.on_deliver(|_, _, _, _| async { unreachable!("the sweep delivers nothing") });
        host.on_extend(|_, _, _, _| async { unreachable!("the sweep publishes nothing") });
        host.on_signal(|_, _, _, _| async { unreachable!("the sweep signals nothing") });
        host.on_catalog(|_| async { unreachable!("the sweep reads no catalog") });
        host.on_gateway_events(|| unreachable!("the sweep watches no gateway"));
        host.handle()
    }

    /// A plugin that registered, with one file kept for each of two sessions.
    fn planted(home: &std::path::Path) -> CheckpointsPlugin {
        let plugin = CheckpointsPlugin::default();
        let mut registrar = Registrar::new(
            MANIFEST.id,
            serde_json::Value::Null,
            Env::rooted(home),
            Default::default(),
        );
        plugin.register(&mut registrar).expect("register");
        let file = home.join("a.txt");
        std::fs::write(&file, b"x").expect("a file");
        let store = plugin.store.get().expect("a store");
        for id in ["ses_here", "ses_gone"] {
            store
                .snapshot(&SessionId::from_raw(id), &TurnId::from_raw("trn_1"), &file)
                .expect("a snapshot");
        }
        plugin
    }

    #[tokio::test]
    async fn starting_collects_the_checkpoints_of_a_session_that_is_no_longer_there() {
        let home = tempfile::tempdir().expect("a temp home");
        let plugin = planted(home.path());
        plugin
            .start(one_session(Some(SessionId::from_raw("ses_here"))))
            .await
            .expect("start");
        assert_eq!(
            plugin.store.get().expect("a store").sessions(),
            ["ses_here"]
        );
    }

    /// An empty listing is an answer: no session exists, so no checkpoint
    /// belongs to one. A store that could not be read answers `Err` instead,
    /// and that is the case where nothing is collected.
    #[tokio::test]
    async fn a_host_that_lists_no_session_has_no_checkpoint_left_to_keep() {
        let home = tempfile::tempdir().expect("a temp home");
        let plugin = planted(home.path());
        plugin.start(one_session(None)).await.expect("start");
        assert!(plugin.store.get().expect("a store").sessions().is_empty());
    }

    #[tokio::test]
    async fn a_host_that_cannot_say_collects_nothing() {
        let home = tempfile::tempdir().expect("a temp home");
        let plugin = planted(home.path());
        plugin
            .start(bingo_sdk::testing::NoHost::handle())
            .await
            .expect("start");
        assert_eq!(
            plugin.store.get().expect("a store").sessions(),
            ["ses_gone", "ses_here"]
        );
    }

    #[tokio::test]
    async fn a_plugin_that_never_registered_starts_without_a_store() {
        CheckpointsPlugin::default()
            .start(bingo_sdk::testing::NoHost::handle())
            .await
            .expect("start");
    }
}
