//! The local platform and its settings slice.
use super::{Config, Loopback};
use crate::adapter::ChannelAdapter;
use crate::limits::{Dialect, Encoding, Limits};
use crate::platform::{Platform as Contract, Prompts};
use crate::secret::Requirement;
use bingo_sdk::Env;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

/// The explicit opt-in required by the packaged binary.
pub const ENABLE_ENV: &str = "BINGO_CHANNEL_LOOPBACK";

pub fn enabled(_env: &Env) -> bool {
    std::env::var_os(ENABLE_ENV).is_some()
}

pub struct Platform;

impl Contract for Platform {
    fn id(&self) -> &'static str {
        Loopback::ID
    }
    fn parse(&self, slice: &Value) -> Result<(), String> {
        serde_json::from_value::<LoopbackChannel>(slice.clone())
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    fn build(&self, slice: &Value, _env: &Env) -> Result<Arc<dyn ChannelAdapter>, String> {
        let settings: LoopbackChannel =
            serde_json::from_value(slice.clone()).map_err(|e| e.to_string())?;
        Ok(Arc::new(Loopback::new(settings.config())))
    }
    fn secret(&self) -> Option<Requirement> {
        None
    }
    fn flag(&self, value: Option<&str>) -> Value {
        json!({"peer": value})
    }
    fn prompts(&self) -> Prompts {
        Prompts {
            settings: &[],
            secret: "",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopbackChannel {
    /// `host:port` to speak NDJSON with. Without one it records and says
    /// nothing, which is what an in-process test wants.
    #[serde(default)]
    pub peer: Option<String>,
    #[serde(default = "yes")]
    pub edits: bool,
    #[serde(default)]
    pub menu: bool,
    #[serde(default = "yes")]
    pub buttons: bool,
    #[serde(default = "yes")]
    pub typing: bool,
    #[serde(default = "yes")]
    pub threads: bool,
    /// Whether this chat will carry a file out.
    #[serde(default = "yes")]
    pub files: bool,
    /// Whether a message gets a sign while it is being worked on.
    #[serde(default = "yes")]
    pub acknowledge: bool,
    /// What a group message must contain for the bot to be addressed.
    #[serde(default = "mention")]
    pub mention: String,
    /// How long one message may be, in characters.
    #[serde(default = "four_thousand")]
    pub max_text: usize,
    #[serde(default = "three")]
    pub max_actions: usize,
}
fn yes() -> bool {
    true
}

fn mention() -> String {
    "@bingo".into()
}

fn four_thousand() -> usize {
    4000
}

fn three() -> usize {
    3
}

impl LoopbackChannel {
    fn config(&self) -> Config {
        Config {
            limits: Limits {
                max_text: (self.max_text, Encoding::Chars),
                dialect: Dialect::Markdown,
                max_actions: self.max_actions,
                max_label: 40,
            },
            menu: self.menu,
            edits: self.edits,
            buttons: self.buttons,
            typing: self.typing,
            threads: self.threads,
            files: self.files,
            acknowledge: self.acknowledge,
            mention: self.mention.clone(),
            peer: self.peer.clone(),
        }
    }
}
