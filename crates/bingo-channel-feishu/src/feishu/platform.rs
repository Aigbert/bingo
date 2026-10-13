//! Settings and setup belong to this platform.
use super::Feishu;
use bingo_channel_adapter::{
    Platform as Contract,
    adapter::ChannelAdapter,
    platform::{Prompt, Prompts},
    secret::Requirement,
};
use bingo_sdk::Env;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};

pub struct Platform;
impl Contract for Platform {
    fn id(&self) -> &'static str {
        Feishu::ID
    }
    fn parse(&self, slice: &Value) -> Result<(), String> {
        serde_json::from_value::<FeishuChannel>(slice.clone())
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    fn build(&self, slice: &Value, env: &Env) -> Result<Arc<dyn ChannelAdapter>, String> {
        let settings: FeishuChannel =
            serde_json::from_value(slice.clone()).map_err(|e| e.to_string())?;
        Ok(Arc::new(Feishu::new(
            bingo_http::client(bingo_http::Bounds::CALL).map_err(|e| e.to_string())?,
            settings.config(env),
        )))
    }
    fn secret(&self) -> Option<Requirement> {
        Some(Requirement {
            id: Feishu::ID,
            variable: Some(APP_SECRET),
        })
    }
    fn flag(&self, value: Option<&str>) -> Value {
        json!({"appId": value})
    }
    fn prompts(&self) -> Prompts {
        Prompts {
            settings: &[Prompt {
                key: "appId",
                label: "App id for feishu (public, goes to the settings): ",
                missing: "an app id is what the platform knows you as",
                name: "app id",
            }],
            secret: "Paste the feishu app secret (not shown): ",
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeishuChannel {
    /// The app id, which is public. The secret is not: it comes from
    /// `BINGO_FEISHU_APP_SECRET`, else the credential store, because a project
    /// settings file is a committed file.
    #[serde(default)]
    pub app_id: Option<String>,
    /// Where the API lives. Lark International is a different host, and the
    /// long connection there is unverified (ADR-0016 consequences).
    #[serde(default)]
    pub base: Option<String>,
    /// Platform commands mapped to terminal controls, such as `/new` to
    /// `/clear` and `/stop` to `esc`.
    #[serde(default)]
    pub command_mappings: BTreeMap<String, String>,
}

/// Where an attachment a Feishu message carried lands, under the data
/// directory this adapter already keeps its lock in (ADR-0051 §2).
const ATTACHMENTS: &str = "attachments";

/// The environment variables a Feishu app is signed with.
pub const APP_ID: &str = "BINGO_FEISHU_APP_ID";
pub const APP_SECRET: &str = "BINGO_FEISHU_APP_SECRET";

impl FeishuChannel {
    /// The settings' app id, else the environment's. The secret is never the
    /// settings': it is the environment's, else the credential store's
    /// (ADR-0020 §8), because a gateway started at boot inherits no shell.
    fn config(&self, env: &Env) -> super::Config {
        super::Config {
            app_id: self
                .app_id
                .clone()
                .or_else(|| std::env::var(APP_ID).ok())
                .unwrap_or_default(),
            app_secret: bingo_channel_adapter::secret::find(env, Feishu::ID, APP_SECRET)
                .map(|found| found.secret)
                .unwrap_or_default(),
            base: self
                .base
                .clone()
                .unwrap_or_else(|| super::api::BASE.to_string()),
            attachments: env
                .data_dir
                .join(bingo_channel_adapter::DIRECTORY)
                .join(Feishu::ID)
                .join(ATTACHMENTS),
            command_mappings: self.command_mappings.clone(),
        }
    }
}
