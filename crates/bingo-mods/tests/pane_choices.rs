#![allow(clippy::unwrap_used, clippy::expect_used)]
use bingo_core::{Host, HostConfig};
use bingo_mods::ModsPlugin;
use bingo_sdk::*;
use futures::StreamExt;
use serde_json::{Value, json};
use std::{path::Path, sync::Arc, time::Duration};

const SOURCE: &str = r#"
export function register(on) {
  on('session.start', async $ => {
    await $.command.register({name:'open-pane',description:'Open',instant:true}, async $ => { await $.ui.open({id:'main',title:'Memory'}); });
    await $.command.register({name:'close-pane',description:'Close',instant:true}, async $ => { await $.ui.close('main'); });
    await $.ui.open({id:'main', title:'Memory'});
  });
  on('ui.render', {component:'Pane'}, () => h('Text', {}, 'Memory'));
}
"#;
async fn start(home: &Path) -> (Arc<Host>, Attachment) {
    let root = home.join("fixture");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("plugin.json"),
        json!({"name":"fixture","version":"1","module":"main.ts","uses":["ui","command"]})
            .to_string(),
    )
    .unwrap();
    std::fs::write(root.join("main.ts"), SOURCE).unwrap();
    let plugin = ModsPlugin::default()
        .with_dirs(vec![root])
        .with_project(home.into());
    let host = Host::build(vec![Box::new(plugin)], HostConfig::new(Env::rooted(home)))
        .await
        .unwrap();
    let attached = host
        .open(
            SessionSelector::Create {
                spec: SessionSpec {
                    cwd: home.into(),
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
    (host, attached)
}
async fn pane(attachment: &mut Attachment) -> Option<bool> {
    tokio::time::timeout(Duration::from_secs(10), async {
        while let Some(frame) = attachment.events.next().await {
            if let Event::Signal {
                plugin, payload, ..
            } = frame.event
                && plugin == "fixture"
            {
                if payload.is_null() {
                    return None;
                }
                if let Some(asked) = payload["slots"][0]["asked"].as_bool() {
                    return Some(asked);
                }
            }
        }
        panic!("pane stream ended")
    })
    .await
    .unwrap()
}
fn submit(attachment: &Attachment, name: &str, args: Value) {
    attachment.handle.submit(
        IntentId::mint(),
        Input::Action {
            action: Action {
                name: name.into(),
                args,
            },
            origin: Some(Origin::surface("tui")),
        },
    );
}

#[tokio::test]
async fn click_and_commands_keep_only_the_persons_choice_across_hosts() {
    let home = tempfile::tempdir().unwrap();
    let path = Env::rooted(home.path()).data_dir.join("pane-choices.json");
    let (host, mut attachment) = start(home.path()).await;
    assert_eq!(pane(&mut attachment).await, Some(false));
    assert!(!path.exists());
    submit(
        &attachment,
        "fixture:ui",
        json!({"handler":"main","event":"open"}),
    );
    assert_eq!(pane(&mut attachment).await, Some(true));
    host.shutdown().await;
    let (host, mut attachment) = start(home.path()).await;
    assert_eq!(pane(&mut attachment).await, Some(true));
    submit(
        &attachment,
        "fixture:ui",
        json!({"handler":"main","event":"close"}),
    );
    assert_eq!(pane(&mut attachment).await, None);
    host.shutdown().await;
    let (host, mut attachment) = start(home.path()).await;
    assert_eq!(pane(&mut attachment).await, Some(false));
    submit(&attachment, "open-pane", Value::Null);
    assert_eq!(pane(&mut attachment).await, Some(true));
    submit(&attachment, "close-pane", Value::Null);
    assert_eq!(pane(&mut attachment).await, None);
    host.shutdown().await;
    std::fs::write(&path, "corrupted").unwrap();
    let (host, mut attachment) = start(home.path()).await;
    assert_eq!(pane(&mut attachment).await, Some(false));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "corrupted");
    host.shutdown().await;
}
