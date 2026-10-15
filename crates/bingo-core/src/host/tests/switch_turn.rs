//! A switch asked for while a turn runs (ADR-0088): `/plugins` is instant, so
//! it is answered at once rather than queued behind the turn, and a plugin
//! the running turn still holds says it lands after that turn.
use std::sync::atomic::{AtomicBool, Ordering};

use bingo_sdk::plugin::{NextStart, PluginSwitch};

use super::commands::{Client, message, rows};
use super::*;

/// A mod with nothing request-shaping, so it switches now — except that the
/// running turn still holds its generation, which the switch finds out and
/// answers with: it is queued behind the turn, and its listing says so until
/// the turn lets it go (as `bingo-mods` does).
#[derive(Default)]
struct HeldByTurn {
    queued: AtomicBool,
}

#[async_trait]
impl PluginSource for HeldByTurn {
    fn id(&self) -> &str {
        "held"
    }
    async fn plugins(&self) -> Vec<PluginStatus> {
        let state = if self.queued.load(Ordering::SeqCst) {
            PluginState::Pending {
                now: true,
                at: NextStart::AfterTurn,
            }
        } else {
            PluginState::On
        };
        vec![PluginStatus {
            id: "held".into(),
            version: "0".into(),
            state,
            from: "held".into(),
            placement: None,
            grant: None,
        }]
    }
    async fn switch_kind(&self, _: &str) -> PluginSwitch {
        PluginSwitch::Now
    }
    async fn switch(&self, _: &str, enabled: bool) -> Result<PluginSwitch, KernelError> {
        self.queued.store(!enabled, Ordering::SeqCst);
        Ok(PluginSwitch::AfterTurn)
    }
}

#[tokio::test]
async fn a_switch_during_a_turn_is_answered_now_and_lands_after_it() {
    let home = tempfile::tempdir().unwrap();
    let source = Arc::new(HeldByTurn::default());
    let host = Host::build(
        vec![TestPlugin::boxed(
            &PROVIDER,
            vec![
                Contribution::Provider(ScriptedProvider::new(vec![Script::Hang(vec![
                    ModelEvent::TextStart { id: "b".into() },
                ])])),
                Contribution::Plugins(source.clone()),
            ],
        )],
        HostConfig::new(env_in(home.path())).with_layer("cli", json!({"model":"m"})),
    )
    .await
    .unwrap();
    let mut client = Client::open(&host).await;
    client.ack("go").await;
    while !client.state.busy() {
        let frame = client.events.next().await.expect("the turn starts");
        client.state.apply(&frame);
    }
    let (ack, _) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        client.ack("/plugins disable held"),
    )
    .await
    .expect("answered while the turn runs, not queued behind it");
    assert_eq!(
        message(&ack),
        "held is off after the running turn; switch queued."
    );
    assert!(client.state.busy(), "the turn went on underneath");
    let (ack, _) = client.ack("/plugins").await;
    let row = rows(&ack)
        .into_iter()
        .find(|row| row[0] == "held")
        .expect("listed");
    assert_eq!(
        (row[2].as_str(), row[3].as_str()),
        ("pending", "off after the running turn")
    );
    host.shutdown().await;
}
