use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static BROKEN: PluginManifest = PluginManifest {
    id: "test.broken",
    version: "0",
    uses: &[],
    provides: &[],
    requires: &[],
    config: None,
};
struct Broken {
    conflict: bool,
    started: Arc<AtomicUsize>,
    stopped: Arc<AtomicUsize>,
}
fn command(name: &'static str) -> Contribution {
    Contribution::Command(ScriptedCommand::new(
        name,
        true,
        Ok(CommandOutcome::Applied { message: None }),
    ))
}
#[async_trait]
impl Plugin for Broken {
    fn manifest(&self) -> &'static PluginManifest {
        &BROKEN
    }
    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        registrar.add(command("partial"));
        if self.conflict {
            registrar.add(command("shared"));
        }
        Ok(())
    }
    async fn start(&self, _: HostHandle) -> Result<(), PluginError> {
        self.started.fetch_add(1, Ordering::SeqCst);
        Err(PluginError::Failed("broken start".into()))
    }
    async fn stop(&self) -> Result<(), PluginError> {
        self.stopped.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
#[tokio::test]
async fn switched_off_conflicts_do_not_abort_or_leak_partial_registration() {
    check(true).await;
}
#[tokio::test]
async fn switched_off_start_failure_does_not_abort_and_is_cleaned_up() {
    check(false).await;
}
async fn check(conflict: bool) {
    let home = tempfile::tempdir().unwrap();
    let started = Arc::new(AtomicUsize::new(0));
    let stopped = Arc::new(AtomicUsize::new(0));
    let plugins: Vec<Box<dyn Plugin>> = vec![
        Box::new(Broken {
            conflict,
            started: started.clone(),
            stopped: stopped.clone(),
        }),
        TestPlugin::boxed(&PROVIDER, vec![command("shared")]),
    ];
    let host = Host::build(
        plugins,
        HostConfig::new(env_in(home.path()))
            .with_layer("cli", json!({"enabledPlugins":{"test.broken":false}})),
    )
    .await
    .unwrap();
    assert!(!host.registry.enabled("test.broken"));
    assert!(
        !host
            .catalog(CatalogKind::Commands)
            .await
            .unwrap()
            .entries
            .iter()
            .any(|row| row.id == "partial")
    );
    assert!(
        host.notices()
            .iter()
            .any(|(_, text)| text.contains("test.broken"))
    );
    host.shutdown().await;
    assert_eq!(started.load(Ordering::SeqCst), usize::from(!conflict));
    assert_eq!(stopped.load(Ordering::SeqCst), usize::from(!conflict));
}
