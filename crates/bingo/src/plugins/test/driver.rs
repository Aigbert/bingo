mod mock;
use super::*;
use bingo_core::{Host, HostConfig};
use bingo_mods::{ModsPlugin, mount::Mount, testing::Environment};
use bingo_provider_fake::{FakePlugin, FakeProvider, Script};
use bingo_sdk::*;
use futures::{FutureExt, StreamExt};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};

pub(super) struct Driver {
    paused: Option<bingo_mods::testing::Paused>,
    home: tempfile::TempDir,
    pub root: PathBuf,
    pub name: String,
    pub host: Arc<Host>,
    modules: ModsPlugin,
    pub session: Attachment,
    environment: Arc<Environment>,
    pub mounts: BTreeMap<String, Mount>,
}
impl Driver {
    pub async fn new(root: &Path, name: &str) -> Result<Self, String> {
        let home = tempfile::tempdir().map_err(error)?;
        let environment = Arc::new(Environment::default());
        let (host, modules, session) =
            build(root, home.path(), environment.clone(), Value::Null).await?;
        Ok(Self {
            paused: None,
            home,
            root: root.into(),
            name: name.into(),
            host,
            modules,
            session,
            environment,
            mounts: BTreeMap::new(),
        })
    }
    pub async fn close(&self) {
        self.host.shutdown().await;
    }
    fn site_value(&self) -> Value {
        json!({"session":self.session.snapshot.summary.id,"cwd":self.session.snapshot.summary.cwd,"plugin":{"name":self.name,"root":self.root}})
    }
    pub fn fold(&mut self, frame: &Frame) {
        if let Event::Invalidated { requests } = &frame.event {
            for mount in self.mounts.values_mut() {
                mount.invalidate(requests);
            }
        }
        self.session.snapshot.apply(frame);
    }
    pub fn drain(&mut self) {
        while let Some(Some(frame)) = self.session.events.next().now_or_never() {
            self.fold(&frame);
        }
    }
    pub async fn wait(&mut self, intent: IntentId, turn: bool) -> Result<(), String> {
        while let Some(frame) = self.session.events.next().await {
            self.fold(&frame);
            match frame.event {
                Event::IntentAck {
                    intent: seen,
                    outcome: IntentOutcome::Rejected { error: why },
                } if seen == intent => return Err(error(why)),
                Event::IntentAck {
                    intent: seen,
                    outcome: IntentOutcome::Applied { .. },
                } if seen == intent => return Ok(()),
                Event::TurnCompleted { status, .. } if turn => {
                    return if status == TurnStatus::Completed {
                        Ok(())
                    } else {
                        Err(format!("turn ended: {status:?}"))
                    };
                }
                Event::InteractionOpened { .. } => {
                    return Err("test requires an unanswered interaction".into());
                }
                _ => (),
            }
        }
        Err("session stream ended".into())
    }
    fn pause(&mut self) {
        if self.paused.is_none() {
            self.paused = Some(bingo_mods::testing::Paused::new());
        }
    }
    pub async fn advance(&mut self, ms: u64) {
        self.pause();
        tokio::time::advance(std::time::Duration::from_millis(ms)).await;
        self.drain();
    }
}
#[async_trait::async_trait]
impl bingo_mods::testing::Driver for Driver {
    fn site(&self) -> Option<Value> {
        Some(self.site_value())
    }
    async fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.drain();
        if let Some(method) = method.strip_prefix("testing/") {
            return self.mock(method, params).await;
        }
        let result = self
            .modules
            .testing_call(&self.name, method, params)
            .await?;
        if matches!(method, "host/prompt/submit" | "host/command/run") {
            self.wait(
                serde_json::from_value(result.clone()).map_err(error)?,
                method == "host/prompt/submit",
            )
            .await?;
        }
        self.drain();
        Ok(result)
    }
}
async fn build(
    root: &Path,
    home: &Path,
    environment: Arc<Environment>,
    options: Value,
) -> Result<(Arc<Host>, ModsPlugin, Attachment), String> {
    let cwd = options
        .get("cwd")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join("project"));
    std::fs::create_dir_all(&cwd).map_err(error)?;
    let script: Script = options
        .get("model")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(error)?
        .unwrap_or_default();
    let modules = ModsPlugin::default()
        .with_dirs(vec![root.into()])
        .with_project(cwd.clone())
        .with_testing(environment);
    let plugins: Vec<Box<dyn Plugin>> = vec![
        Box::new(FakePlugin::new(Arc::new(FakeProvider::new(script)))),
        Box::new(bingo_tool_fs::FsPlugin),
        Box::new(bingo_tool_bash::BashPlugin::default()),
        Box::new(modules.clone()),
    ];
    let config = HostConfig::new(Env::rooted(home))
        .with_layer("test", json!({"provider":"fake","model":"fake-1"}));
    let host = Host::build(plugins, config).await.map_err(error)?;
    admitted(&host, root).await?;
    let session = host
        .open(
            SessionSelector::Create {
                spec: SessionSpec {
                    cwd,
                    ..Default::default()
                },
            },
            ClientIdentity {
                name: "plugin-kit".into(),
                surface: "terminal".into(),
            },
            OpenOptions::default(),
        )
        .await
        .map_err(error)?;
    Ok((host, modules, session))
}
async fn admitted(host: &Arc<Host>, root: &Path) -> Result<(), String> {
    let name = bingo_plugin_rpc::validate::read(root).map_err(error)?.name;
    let catalogue = host.catalog(CatalogKind::Plugins).await.map_err(error)?;
    if !catalogue
        .entries
        .iter()
        .any(|entry| entry.id == name && entry.meta["enabled"] == true)
    {
        let reason = catalogue
            .entries
            .iter()
            .find(|entry| entry.id == name)
            .map(|entry| entry.meta["reason"].to_string())
            .unwrap_or_else(|| "module was not discovered".into());
        host.shutdown().await;
        return Err(format!("{name}: {reason}"));
    }
    Ok(())
}
pub(super) fn error(why: impl std::fmt::Display) -> String {
    why.to_string()
}
