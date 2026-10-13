#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
mod offline;
use bingo_core::{Host, HostConfig};
use bingo_mods::ModsPlugin;
use bingo_sdk::*;
use futures::StreamExt;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};

pub struct Fixture {
    pub home: tempfile::TempDir,
    pub root: PathBuf,
    pub plugin: ModsPlugin,
    pub host: Arc<Host>,
    pub attachment: Attachment,
}
impl Fixture {
    pub async fn new(source: &str, uses: &[&str], options: Value) -> Self {
        Self::build(source, uses, options, false).await
    }
    pub async fn agent(source: &str, uses: &[&str], options: Value) -> Self {
        Self::build(source, uses, options, true).await
    }
    async fn build(source: &str, uses: &[&str], options: Value, agent: bool) -> Self {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().join("fixture");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(
            root.join("plugin.json"),
            json!({"name":"fixture", "version":"1", "module":"main.ts", "uses":uses}).to_string(),
        )
        .unwrap();
        std::fs::write(root.join("main.ts"), source).unwrap();
        let plugin = ModsPlugin::default().with_dirs(vec![root.clone()]);
        let config = HostConfig::new(Env::rooted(home.path()))
            .with_layer("test", options)
            .with_layer("offline", json!({"provider":"offline","model":"offline"}));
        let mut plugins: Vec<Box<dyn Plugin>> = vec![Box::new(plugin.clone())];
        if agent {
            plugins.push(Box::new(offline::Offline::default()));
        }
        let host = Host::build(plugins, config).await.unwrap();
        let attachment = host
            .open(
                SessionSelector::Create {
                    spec: SessionSpec {
                        cwd: home.path().into(),
                        driver: if agent {
                            Driver::default()
                        } else {
                            Driver::Log
                        },
                        ..Default::default()
                    },
                },
                ClientIdentity {
                    name: "fixture".into(),
                    surface: "test".into(),
                },
                OpenOptions::default(),
            )
            .await
            .unwrap();
        Self {
            home,
            root,
            plugin,
            host,
            attachment,
        }
    }
    pub async fn call(&self, noun: &'static str, method: &str, input: Value) -> Value {
        if self.host.service_wire(noun).is_none() {
            self.host.open_service(noun, Arc::new(Echo(noun))).unwrap();
        }
        self.host
            .service_wire(noun)
            .unwrap()
            .call(method, input)
            .await
            .unwrap()
    }
    pub async fn notices(&mut self) -> Vec<String> {
        self.host
            .extend(&self.attachment.session, "test", "barrier", Value::Null)
            .await
            .unwrap();
        let mut notices = Vec::new();
        while let Some(frame) = self.attachment.events.next().await {
            match frame.event {
                Event::Notice { text, .. } => notices.push(text),
                Event::ItemCompleted { item } => {
                    if let ItemBody::Notice { text, .. } = item.body {
                        notices.push(text);
                    }
                }
                Event::Extension { kind, .. } if kind == "barrier" => break,
                _ => (),
            }
        }
        notices
    }
}
struct Echo(&'static str);
#[async_trait::async_trait]
impl WireService for Echo {
    fn face(&self) -> Option<Face> {
        Some(
            Face::new(self.0)
                .method::<Value, Value>("echo", false)
                .method::<Value, Value>("call", false)
                .method::<Value, Value>("check", false)
                .method::<Value, Value>("complete", false),
        )
    }
    async fn call(&self, _: &str, input: Value) -> Result<Value, ServiceError> {
        Ok(input)
    }
}
