//! Reconcile only names this installation wrote, under one app-wide lock.
use super::{Api, ApiError, Feishu};
use async_trait::async_trait;
use bingo_channel_adapter::{
    adapter::{Menu, MenuEntry},
    error::ChannelError,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

const PATH: &str = "/open-apis/application/v7/app_slash_commands";
#[derive(Default)]
struct State {
    disabled: bool,
    warned: BTreeSet<String>,
}
pub(super) struct Publisher {
    path: PathBuf,
    state: tokio::sync::Mutex<State>,
}

impl Publisher {
    pub fn new(attachments: &Path, app: &str) -> Self {
        let app: String = app.bytes().map(|b| format!("{b:02x}")).collect();
        Self {
            path: attachments
                .parent()
                .unwrap_or(attachments)
                .join(app)
                .join("slash-commands.json"),
            state: Default::default(),
        }
    }

    pub async fn publish(&self, api: &Api, entries: &[MenuEntry]) -> Result<(), ChannelError> {
        let mut state = self.state.lock().await;
        if state.disabled {
            return Ok(());
        }
        let result = self.sync(api, entries, &mut state).await;
        if let Err(error) = &result
            && missing_scope(error)
        {
            state.disabled = true;
        }
        result.map_err(Into::into)
    }

    async fn sync(
        &self,
        api: &Api,
        entries: &[MenuEntry],
        state: &mut State,
    ) -> Result<(), ApiError> {
        let mut owned = self.read()?;
        let mut remote = list(api).await?;
        // A reused name with a different id belongs to the person now.
        owned.retain(|name, id| {
            remote
                .get(name)
                .is_some_and(|item| item["command_id"] == *id)
        });
        self.save(&owned)?;
        self.remove(api, entries, &mut owned, &mut remote).await?;
        for entry in entries.iter().take(100) {
            self.upsert(api, entry, &mut owned, &mut remote, state)
                .await?;
        }
        Ok(())
    }

    async fn remove(
        &self,
        api: &Api,
        entries: &[MenuEntry],
        owned: &mut BTreeMap<String, String>,
        remote: &mut BTreeMap<String, Value>,
    ) -> Result<(), ApiError> {
        let removed: Vec<_> = owned
            .iter()
            .filter(|(name, _)| !entries.iter().take(100).any(|e| &e.name == *name))
            .map(|(n, id)| (n.clone(), id.clone()))
            .collect();
        for (name, id) in removed {
            api.delete(&format!("{PATH}/{}", segment(&id))).await?;
            owned.remove(&name);
            remote.remove(&name);
            self.save(owned)?;
        }
        Ok(())
    }

    async fn upsert(
        &self,
        api: &Api,
        entry: &MenuEntry,
        owned: &mut BTreeMap<String, String>,
        remote: &mut BTreeMap<String, Value>,
        state: &mut State,
    ) -> Result<(), ApiError> {
        if let Some(item) = remote.get(&entry.name) {
            if let Some(id) = owned.get(&entry.name) {
                if item["description"]["default_value"] != entry.description {
                    api.patch(
                        &format!("{PATH}/{}", segment(id)),
                        json!({"description":description(entry)}),
                    )
                    .await?;
                }
            } else {
                warn(
                    state,
                    &entry.name,
                    "the command menu keeps a name owned by the person",
                );
            }
            return Ok(());
        }
        if remote.len() >= 100 {
            warn(state, "limit", "the app command menu is full");
            return Ok(());
        }
        let body = json!({"command":entry.name,"description":description(entry)});
        let created = match api.post(PATH, body).await {
            Err(ApiError::Refused {
                code: 40000000,
                message,
            }) if message.contains("command already exists") => {
                warn(
                    state,
                    &entry.name,
                    "the command menu keeps a name created elsewhere",
                );
                return Ok(());
            }
            result => result?,
        };
        let id = created["data"]["command_id"]
            .as_str()
            .ok_or_else(|| ApiError::Transport("feishu created no command id".into()))?;
        owned.insert(entry.name.clone(), id.into());
        self.save(owned)?;
        remote.insert(
            entry.name.clone(),
            json!({"command_id":id,"description":description(entry)}),
        );
        Ok(())
    }

    fn read(&self) -> Result<BTreeMap<String, String>, ApiError> {
        match std::fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| ApiError::Transport(format!("command menu ownership: {e}"))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
            Err(e) => Err(ApiError::Transport(format!("command menu ownership: {e}"))),
        }
    }

    fn save(&self, owned: &BTreeMap<String, String>) -> Result<(), ApiError> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| ApiError::Transport("command menu has no directory".into()))?;
        std::fs::create_dir_all(parent)
            .and_then(|()| std::fs::write(&self.path, json!(owned).to_string()))
            .map_err(|e| ApiError::Transport(format!("command menu ownership: {e}")))
    }
}

async fn list(api: &Api) -> Result<BTreeMap<String, Value>, ApiError> {
    let answer = api.get(PATH).await?;
    let items = answer["data"]["items"]
        .as_array()
        .ok_or_else(|| ApiError::Transport("feishu listed no command items".into()))?;
    items
        .iter()
        .map(|item| {
            item["command"]
                .as_str()
                .zip(item["command_id"].as_str())
                .map(|(name, _)| (name.into(), item.clone()))
                .ok_or_else(|| ApiError::Transport("feishu listed an incomplete command".into()))
        })
        .collect()
}
fn description(entry: &MenuEntry) -> Value {
    json!({"default_value":entry.description,"i18n":{}})
}
fn segment(value: &str) -> String {
    value
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"_-".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
fn missing_scope(error: &ApiError) -> bool {
    matches!(
        error,
        ApiError::Refused {
            code: 403 | 99991672 | 99991663,
            ..
        }
    )
}
fn warn(state: &mut State, name: &str, message: &str) {
    if state.warned.insert(name.into()) {
        tracing::warn!(%name, "{message}");
    }
}
#[async_trait]
impl Menu for Feishu {
    async fn publish(&self, entries: &[MenuEntry]) -> Result<(), ChannelError> {
        self.menu.publish(&self.api, entries).await
    }
}
#[cfg(test)]
mod tests;
