#![allow(clippy::unwrap_used, clippy::expect_used)]
use bingo_core::{Host, HostConfig};
use bingo_mods::ModsPlugin;
use bingo_sdk::*;
use futures::StreamExt;
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

struct Running {
    home: tempfile::TempDir,
    project: tempfile::TempDir,
    host: Arc<Host>,
    attachment: Attachment,
}
impl Running {
    async fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let plugin = ModsPlugin::default().with_project(project.path().into());
        let host = Host::build(
            vec![Box::new(plugin)],
            HostConfig::new(Env::rooted(home.path())),
        )
        .await
        .unwrap();
        let attachment = host
            .open(
                SessionSelector::Create {
                    spec: SessionSpec {
                        cwd: project.path().into(),
                        driver: Driver::Log,
                        ..Default::default()
                    },
                },
                ClientIdentity {
                    name: "test".into(),
                    surface: "test".into(),
                },
                OpenOptions::default(),
            )
            .await
            .unwrap();
        Self {
            home,
            project,
            host,
            attachment,
        }
    }
    fn user(&self) -> PathBuf {
        self.home.path().join(".bingo/plugins")
    }
    fn project(&self) -> PathBuf {
        self.project.path().join(".bingo/plugins")
    }
    async fn reload(&mut self) -> String {
        let intent = IntentId::mint();
        self.attachment.handle.submit(
            intent.clone(),
            Input::text("/plugins reload", Origin::surface("test")),
        );
        tokio::time::timeout(Duration::from_secs(15), async {
            while let Some(frame) = self.attachment.events.next().await {
                if let Event::IntentAck {
                    intent: ack,
                    outcome,
                } = frame.event
                    && ack == intent
                {
                    return serde_json::to_value(outcome).unwrap().to_string();
                }
            }
            panic!("no reload answer")
        })
        .await
        .unwrap()
    }
    async fn notice(&mut self, code: &str, name: &str) {
        tokio::time::timeout(Duration::from_secs(15), async {
            while let Some(frame) = self.attachment.events.next().await {
                if let Event::ItemCompleted { item } = frame.event
                    && matches!(item.body, ItemBody::Notice { code: got, text, .. } if got == code && text.contains(name)) { return; }
            }
            panic!("notice stream ended")
        }).await.unwrap();
    }
    async fn listed(&self, name: &str) -> Option<Value> {
        self.host
            .catalog(CatalogKind::Plugins)
            .await
            .unwrap()
            .entries
            .into_iter()
            .find(|row| row.id == name)
            .map(|row| row.meta)
    }
}
fn fixture(dir: &Path, name: &str, source: &str) -> PathBuf {
    let root = dir.join(name);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.ts"), source).unwrap();
    std::fs::write(
        root.join("plugin.json"),
        json!({"name":name,"version":"1","module":"main.ts","uses":["state","store","ui"]})
            .to_string(),
    )
    .unwrap();
    root
}
const SOURCE: &str = r#"export function register(on) {
  on('session.start', async $ => {
    if (!await $.store.get('kept')) await $.store.set('kept', 'yes');
    const old = await $.state.get('starts');
    await $.state.set('starts', (old?.value || 0) + 1);
    await $.ui.open({id:'main',title:'Rescan'});
  });
}"#;

#[tokio::test]
async fn explicit_reload_adds_removes_replaces_and_isolates_broken_mods() {
    let mut run = Running::new().await;
    fixture(&run.user(), "user", SOURCE);
    let project = fixture(&run.project(), "project", SOURCE);
    let answer = run.reload().await;
    assert!(
        answer.contains("added") && answer.contains("project"),
        "{answer}"
    );
    assert!(run.listed("user").await.unwrap()["enabled"] == true);
    assert!(run.listed("project").await.unwrap()["enabled"] == true);
    assert!(run.reload().await.contains("nothing changed"));
    std::fs::write(project.join("main.ts"), format!("{SOURCE}\n// changed")).unwrap();
    fixture(
        &run.project(),
        "broken",
        "export function register( { syntax error",
    );
    let answer = run.reload().await;
    assert!(
        answer.contains("1 reloaded") && answer.contains("broken") && answer.contains("errors:"),
        "{answer}"
    );
    assert!(run.listed("user").await.unwrap()["enabled"] == true);
    assert!(run.listed("broken").await.unwrap()["enabled"] == false);
    let store =
        bingo_plugin_host::state::Store::new(&Env::rooted(run.home.path()).data_dir, "project");
    assert_eq!(
        store
            .get(run.attachment.session.to_string(), "starts".into())
            .await
            .unwrap()
            .unwrap()
            .value,
        json!(2)
    );
    let kept = store
        .get("global".into(), "kept".into())
        .await
        .unwrap()
        .unwrap();
    assert_eq!((kept.version, kept.value), (1, json!("yes")));

    std::fs::remove_dir_all(project).unwrap();
    assert!(run.reload().await.contains("1 removed"));
    assert!(run.listed("project").await.is_none());
    let memory = Env::rooted(run.home.path())
        .data_dir
        .join("pane-choices.json");
    assert!(!memory.exists(), "reload is not a request to open a pane");
    run.host.shutdown().await;
}

#[tokio::test]
async fn user_add_edit_remove_are_watched_and_project_edits_wait() {
    let mut run = Running::new().await;
    let project = fixture(&run.project(), "project", SOURCE);
    run.reload().await;
    std::fs::write(
        project.join("main.ts"),
        format!("{SOURCE}\n// explicit only"),
    )
    .unwrap();
    fixture(&run.project(), "later-project", SOURCE);
    let user = fixture(&run.user(), "watched", SOURCE);
    run.notice("PLUGIN_RELOADED", "watched").await;
    assert!(run.listed("watched").await.unwrap()["enabled"] == true);
    assert!(run.listed("later-project").await.is_none());
    let shadow = fixture(&run.project(), "watched", SOURCE);
    std::fs::write(user.join("main.ts"), format!("{SOURCE}\n// watched edit")).unwrap();
    run.notice("PLUGIN_RELOADED", "watched").await;
    assert_eq!(run.listed("watched").await.unwrap()["origin"], "user");
    std::fs::remove_dir_all(shadow).unwrap();
    std::fs::remove_dir_all(user).unwrap();
    run.notice("PLUGIN_UNLOADED", "watched").await;
    assert!(run.listed("watched").await.is_none());
    let answer = run.reload().await;
    assert!(
        answer.contains("1 added") && answer.contains("1 reloaded"),
        "{answer}"
    );
    run.host.shutdown().await;
}

#[tokio::test]
async fn refused_generation_stays_refused_after_reload() {
    let mut run = Running::new().await;
    fixture(
        &run.user(),
        "a-policy",
        "export function register(on) { on('plugin.register', {name:'target'}, () => ({reason:'refused by policy'})); }",
    );
    run.reload().await;
    let target = fixture(&run.project(), "target", SOURCE);
    assert!(run.reload().await.contains("refused by policy"));
    assert!(run.listed("target").await.unwrap()["enabled"] == false);
    std::fs::write(
        target.join("main.ts"),
        format!("{SOURCE}\n// still refused"),
    )
    .unwrap();
    assert!(run.reload().await.contains("refused by policy"));
    assert!(run.listed("target").await.unwrap()["enabled"] == false);
    run.host.shutdown().await;
}
