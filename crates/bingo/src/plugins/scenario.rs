//! Run scenario files with a real kernel and a scripted, offline provider.
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use bingo_core::{Host, HostConfig};
use bingo_plugin_rpc::PluginRpcPlugin;
use bingo_provider_fake::{FakePlugin, FakeProvider};
use bingo_sdk::{
    Attachment, ClientIdentity, Env, Event, HostApi, Input, IntentId, KernelError, OpenOptions,
    Origin, Plugin, SessionSelector, SessionSpec,
};
use futures::{FutureExt, StreamExt};

use super::{
    expect, invalid,
    schema::{Scenario, Step},
};

pub async fn run(root: &Path) -> Result<(), KernelError> {
    let root = root.canonicalize().map_err(invalid)?;
    bingo_plugin_rpc::validate::read(&root)?;
    let mut paths = std::fs::read_dir(root.join("scenarios"))
        .map_err(invalid)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(invalid)?;
    paths.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "json")
    });
    paths.sort();
    if paths.is_empty() {
        return Err(invalid("scenarios: no .json scenarios found"));
    }
    for path in paths {
        let raw = std::fs::read_to_string(&path).map_err(invalid)?;
        let scenario: Scenario = serde_json::from_str(&raw)
            .map_err(|error| invalid(format!("{}: {error}", path.display())))?;
        execute(&root, scenario)
            .await
            .map_err(|error| invalid(format!("{}: {}", path.display(), error.message)))?;
        println!("PASS {}", path.display());
    }
    Ok(())
}

async fn execute(root: &Path, scenario: Scenario) -> Result<(), KernelError> {
    let home = tempfile::tempdir().map_err(invalid)?;
    let plugin = PluginRpcPlugin::default().with_project(home.path().into());
    let plugins: Vec<Box<dyn Plugin>> = vec![
        Box::new(FakePlugin::new(Arc::new(FakeProvider::new(scenario.model)))),
        Box::new(bingo_tool_fs::FsPlugin),
        Box::new(bingo_tool_bash::BashPlugin::default()),
        Box::new(plugin.clone()),
    ];
    let config = HostConfig::new(Env::rooted(home.path())).with_layer(
        "scenario",
        serde_json::json!({
            "provider":"fake", "model":"fake-1", "pluginDirs":[root],
        }),
    );
    let host = Host::build(plugins, config).await.map_err(invalid)?;
    let result = drive(&host, &plugin, root, scenario.steps).await;
    host.shutdown().await;
    result
}

async fn drive(
    host: &Arc<Host>,
    plugin: &PluginRpcPlugin,
    root: &Path,
    steps: Vec<Step>,
) -> Result<(), KernelError> {
    let name = bingo_plugin_rpc::validate::read(root)?.name;
    let catalog = host.catalog(bingo_sdk::CatalogKind::Plugins).await?;
    let status = catalog
        .entries
        .iter()
        .find(|entry| entry.id == name)
        .ok_or_else(|| invalid("plugin was not discovered"))?;
    if status.meta["enabled"] != true {
        return Err(invalid(format!("plugin {name}: {}", status.meta["reason"])));
    }
    let mut session = host
        .open(
            SessionSelector::Create {
                spec: SessionSpec {
                    cwd: root.into(),
                    ..Default::default()
                },
            },
            ClientIdentity {
                name: "plugin-kit".into(),
                surface: "test".into(),
            },
            OpenOptions::default(),
        )
        .await?;
    let mut notices = Vec::new();
    for (index, step) in steps.into_iter().enumerate() {
        let result = step_run(step, &mut session, &mut notices, plugin, root).await;
        if let Err(error) = result {
            return Err(invalid(format!(
                "step {}: {}; {}",
                index + 1,
                error.message,
                expect::shown(&session.snapshot, &notices)
            )));
        }
    }
    Ok(())
}

async fn step_run(
    step: Step,
    session: &mut Attachment,
    notices: &mut Vec<String>,
    plugin: &PluginRpcPlugin,
    root: &Path,
) -> Result<(), KernelError> {
    match step {
        Step::Submit { text } => submit(session, notices, text).await,
        Step::Reload {} => plugin.reload(root).await,
        Step::Expect(expected) => {
            while let Some(Some(frame)) = session.events.next().now_or_never() {
                fold(session, notices, &frame);
            }
            // Notices are delivered by the host's owned drain. Wait on its event,
            // never on a guessed sleep between reload and an expectation.
            if matches!(expected, super::schema::Expectation::Notice { .. }) {
                let waiting = async {
                    while !expect::satisfied(&expected, &session.snapshot, notices) {
                        let frame = session
                            .events
                            .next()
                            .await
                            .ok_or_else(|| invalid("session stream ended"))?;
                        fold(session, notices, &frame);
                    }
                    Ok::<_, KernelError>(())
                };
                tokio::time::timeout(Duration::from_secs(10), waiting)
                    .await
                    .map_err(|_| invalid("notice deadline exceeded"))??;
            }
            if expect::satisfied(&expected, &session.snapshot, notices) {
                Ok(())
            } else {
                Err(invalid(format!("expectation not met: {expected:?}")))
            }
        }
    }
}

fn fold(session: &mut Attachment, notices: &mut Vec<String>, frame: &bingo_sdk::Frame) {
    if let Event::Notice { text, .. } = &frame.event {
        notices.push(text.clone());
    }
    session.snapshot.apply(frame);
}

async fn submit(
    session: &mut Attachment,
    notices: &mut Vec<String>,
    text: String,
) -> Result<(), KernelError> {
    let intent = IntentId::mint();
    session.handle.submit(
        intent.clone(),
        Input::text(text, Origin::surface("plugin-kit")),
    );
    let waiting = async {
        while let Some(frame) = session.events.next().await {
            fold(session, notices, &frame);
            match &frame.event {
                Event::TurnCompleted {
                    status: bingo_sdk::TurnStatus::Completed,
                    ..
                } => return Ok(()),
                Event::TurnCompleted { status, .. } => {
                    return Err(invalid(format!("turn ended: {status:?}")));
                }
                Event::IntentAck {
                    intent: seen,
                    outcome: bingo_sdk::IntentOutcome::Rejected { error },
                } if seen == &intent => return Err(error.clone()),
                Event::InteractionOpened { .. } => {
                    return Err(invalid(
                        "scenario requires a permission answer; add a plugin denial or use a read-only call",
                    ));
                }
                _ => {}
            }
        }
        Err(invalid("session stream ended before turn completion"))
    };
    tokio::time::timeout(Duration::from_secs(30), waiting)
        .await
        .map_err(|_| invalid("turn deadline exceeded"))?
}
