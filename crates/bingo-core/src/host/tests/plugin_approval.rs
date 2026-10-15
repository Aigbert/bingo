//! `/plugins approve <name> <tier>`: the trust tier is written only once
//! the approval itself stands.

use super::commands::{Client, message};
use super::*;
use crate::settings;

/// A repository plugin `p` waiting for a yes, listed under the name it
/// waits with.
struct Waiting {
    request: bingo_sdk::Approval,
    listed: bool,
}
#[async_trait]
impl bingo_sdk::PluginSource for Waiting {
    fn id(&self) -> &str {
        "discovered"
    }
    async fn plugins(&self) -> Vec<PluginStatus> {
        if !self.listed {
            return vec![];
        }
        vec![PluginStatus {
            id: "plugins:p".into(),
            version: "1".into(),
            state: PluginState::Waiting { changed: false },
            from: "discovered".into(),
            placement: Some(PluginPlacement {
                origin: PluginOrigin::Project,
                runtime: PluginRuntime::Module,
            }),
            grant: None,
        }]
    }
    async fn approval(&self, name: &str) -> Option<bingo_sdk::Approval> {
        (self.request.what == format!("plugin:{name}")).then(|| self.request.clone())
    }
}

/// A host whose repository holds `p`, its directory present or not.
async fn scene(
    home: &std::path::Path,
    repo: &std::path::Path,
    present: bool,
    listed: bool,
) -> Arc<Host> {
    if present {
        let dir = repo.join(".bingo/plugins/p");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plugin.json"), "{}").unwrap();
    }
    let request = bingo_sdk::Approval {
        root: repo.into(),
        what: "plugin:p".into(),
    };
    Host::build(
        vec![TestPlugin::boxed(
            &PROVIDER,
            vec![
                Contribution::Provider(ScriptedProvider::new(vec![])),
                Contribution::Plugins(Arc::new(Waiting { request, listed })),
            ],
        )],
        HostConfig::new(env_in(home)).with_layer("cli", json!({"model":"m"})),
    )
    .await
    .unwrap()
}
fn saved(home: &std::path::Path) -> serde_json::Map<String, serde_json::Value> {
    settings::read_document(&env_in(home).user_settings()).unwrap()
}

#[tokio::test]
async fn an_approval_that_fails_writes_no_trust() {
    let (home, repo) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let host = scene(home.path(), repo.path(), false, true).await;
    let mut client = Client::open(&host).await;
    let (ack, _) = client.ack("/plugins approve p full").await;
    assert!(!matches!(ack, IntentOutcome::Applied { .. }), "{ack:?}");
    assert!(saved(home.path()).get("pluginTrust").is_none());
}

#[tokio::test]
async fn an_approval_with_a_tier_keeps_both() {
    let (home, repo) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let host = scene(home.path(), repo.path(), true, true).await;
    let mut client = Client::open(&host).await;
    let (ack, _) = client.ack("/plugins approve p full").await;
    assert!(message(&ack).contains("trust: full"), "{ack:?}");
    let saved = saved(home.path());
    assert_eq!(saved["pluginTrust"]["p"]["tier"], "full");
    assert!(saved["approved"].is_object());
}

#[tokio::test]
async fn trust_that_cannot_be_saved_is_said_after_the_approval() {
    let (home, repo) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let host = scene(home.path(), repo.path(), true, false).await;
    let mut client = Client::open(&host).await;
    let (ack, _) = client.ack("/plugins approve p full").await;
    let said = message(&ack);
    assert!(said.starts_with("approved p"), "{said}");
    assert!(said.contains("trust was not saved"), "{said}");
    assert!(said.contains("run /plugins trust p full"), "{said}");
    let saved = saved(home.path());
    assert!(saved["approved"].is_object());
    assert!(saved.get("pluginTrust").is_none());
}

/// The verb is read the way its guard reads it: a tab or a doubled space
/// still approves, rather than falling through to a switch.
#[tokio::test]
async fn an_approval_reads_its_verb_at_any_whitespace() {
    let (home, repo) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let host = scene(home.path(), repo.path(), true, true).await;
    let mut client = Client::open(&host).await;
    let (ack, _) = client.ack("/plugins approve\tp  full").await;
    assert!(message(&ack).contains("trust: full"), "{ack:?}");
    assert_eq!(saved(home.path())["pluginTrust"]["p"]["tier"], "full");
}
