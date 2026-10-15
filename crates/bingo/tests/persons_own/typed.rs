use super::*;
use bingo_provider_fake::{FakePlugin, FakeProvider};
use std::sync::Arc;

struct DeferredShell;

impl Plugin for DeferredShell {
    fn manifest(&self) -> &'static PluginManifest {
        static MANIFEST: PluginManifest = PluginManifest {
            id: "test.shell",
            version: "1",
            uses: &[],
            requires: &[],
            provides: &[],
            config: None,
        };
        &MANIFEST
    }

    fn register(&self, r: &mut Registrar) -> Result<(), PluginError> {
        r.add(Contribution::Command(Arc::new(Self)));
        Ok(())
    }
}

#[async_trait::async_trait]
impl Command for DeferredShell {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            instant: false,
            ..bingo_tool_bash::ShellCommand.spec()
        }
    }

    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        bingo_tool_bash::ShellCommand.run(args, cx).await
    }
}

// The shipped shell is instant. Defer the same implementation to exercise
// the kernel's queued command path without changing production scheduling.
#[tokio::test]
async fn every_person_surface_can_run_a_queued_typed_shell_line() {
    for surface in PERSON_SURFACES {
        let home = tempfile::tempdir().unwrap();
        let provider = Arc::new(FakeProvider::new(serde_json::from_value(script()).unwrap()));
        let config = bingo_core::HostConfig::new(Env::rooted(home.path()))
            .with_layer("fixture", json!({"provider":"fake","model":"fake-1"}));
        let host = bingo_core::Host::build(
            vec![Box::new(FakePlugin::new(provider)), Box::new(DeferredShell)],
            config,
        )
        .await
        .unwrap();
        let mut session = host
            .open(create(home.path().into()), who(), OpenOptions::default())
            .await
            .unwrap();
        let intent = IntentId::mint();
        session.handle.submit(
            intent.clone(),
            Input::text("Keep working.", Origin::surface(surface)),
        );
        assert!(matches!(
            ack_for(&mut session, &intent).await,
            IntentOutcome::TurnStarted { .. }
        ));
        let queued = IntentId::mint();
        session.handle.submit(
            queued.clone(),
            Input::text("!touch typed-marker", Origin::surface(surface)),
        );
        assert!(
            matches!(
                ack_for(&mut session, &queued).await,
                IntentOutcome::Queued { .. }
            ),
            "{surface}"
        );
        session
            .handle
            .interrupt(IntentId::mint(), InterruptScope::Head);
        let outcome = ack_for(&mut session, &queued).await;
        assert!(
            matches!(outcome, IntentOutcome::Applied { .. }),
            "{surface}: {outcome:?}"
        );
        assert!(home.path().join("typed-marker").exists(), "{surface}");
        host.shutdown().await;
    }
}
