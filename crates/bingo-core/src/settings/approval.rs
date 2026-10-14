//! Provenance of runnable entries survives the settings fold.
use super::{Claim, Layer, SettingsError};
use crate::approval::{Approved, digest_entry, verdict};
use bingo_sdk::{PluginState, Waiting};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

type Entries = BTreeMap<(String, String), String>;
pub(super) fn entries(
    layers: &[Layer],
    claims: &[Claim],
    merged: &Map<String, Value>,
) -> Result<Entries, SettingsError> {
    let mut entries = BTreeMap::new();
    for layer in layers.iter().filter(|layer| layer.repository().is_some()) {
        let Some(root) = layer.repository() else {
            continue;
        };
        let root = root
            .to_str()
            .ok_or_else(|| SettingsError::Type {
                key: "approved".into(),
                layer: layer.source.clone(),
                message: "non-UTF-8 repository paths cannot be approved".into(),
            })?
            .to_string();
        for key in claims.iter().flat_map(|claim| &claim.approve) {
            for name in layer
                .value
                .get(key)
                .and_then(Value::as_object)
                .into_iter()
                .flat_map(|map| map.keys())
            {
                if let Some(value) = merged.get(key).and_then(|entries| entries.get(name)) {
                    entries.insert((root.clone(), format!("{key}:{name}")), digest_entry(value));
                }
            }
        }
    }
    Ok(entries)
}
pub(super) fn waiting(
    plugins: &mut BTreeMap<String, Value>,
    claims: &[Claim],
    entries: &Entries,
    approved: &Approved,
) {
    for claim in claims.iter().filter(|claim| !claim.approve.is_empty()) {
        let mut waiting = Vec::new();
        for ((root, what), value) in entries {
            let Some((key, name)) = what.split_once(':') else {
                continue;
            };
            if !claim.approve.iter().any(|claimed| claimed == key) {
                continue;
            }
            if let PluginState::Waiting { changed } = verdict(approved, root, what, value) {
                waiting.push(Waiting {
                    root: root.into(),
                    key: key.into(),
                    name: name.into(),
                    changed,
                });
            }
        }
        if let Some(Value::Object(slice)) = plugins.get_mut(&claim.plugin) {
            slice.insert("waiting".into(), serde_json::json!(waiting));
        }
    }
}
pub(super) fn ignored(layers: &[Layer]) -> impl Iterator<Item = (String, String)> + '_ {
    layers
        .iter()
        .filter(|layer| !layer.is_user() && layer.value.contains_key("approved"))
        .map(|layer| {
            (
                "APPROVED_IGNORED".into(),
                format!(
                    "approved in {} is ignored; approvals belong in user settings",
                    layer.source
                ),
            )
        })
}

pub(super) fn personal(layers: &[Layer], claims: &[Claim]) -> (Vec<Layer>, Vec<(String, String)>) {
    let mut layers = layers.to_vec();
    let mut notices = Vec::new();
    for layer in layers
        .iter_mut()
        .filter(|layer| layer.repository().is_some())
    {
        for (key, _) in claims
            .iter()
            .flat_map(|claim| &claim.keys)
            .filter(|(_, mode)| *mode == bingo_sdk::Merge::PersonalAccumulate)
        {
            if remove(&mut layer.value, key) {
                notices.push(("PROJECT_SETTING_IGNORED".into(), format!("{key} in {} is ignored; executable directories must be supplied by the person", layer.source)));
            }
        }
    }
    (layers, notices)
}
fn remove(object: &mut Map<String, Value>, path: &str) -> bool {
    if let Some((head, rest)) = path.split_once('.') {
        object
            .get_mut(head)
            .and_then(Value::as_object_mut)
            .is_some_and(|child| remove(child, rest))
    } else {
        object.remove(path).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn home_settings_are_personal_even_when_home_is_the_working_directory() {
        let home = tempfile::tempdir().unwrap();
        let env = bingo_sdk::Env::rooted(home.path());
        std::fs::create_dir_all(&env.config_dir).unwrap();
        std::fs::write(
            env.user_settings(),
            "[mcpServers.personal]\ncommand = 'never-run'\n",
        )
        .unwrap();
        let layers = super::super::load(&env, home.path(), None).unwrap();
        assert_eq!(layers.len(), 1);
        let claim = Claim {
            plugin: "test".into(),
            keys: vec![("mcpServers".into(), bingo_sdk::Merge::ByName)],
            approve: vec!["mcpServers".into()],
        };
        let merged = super::super::merge(&layers, &[claim]).unwrap();
        assert_eq!(merged.plugins["test"]["waiting"], json!([]));
    }

    #[test]
    fn executable_directory_aliases_are_never_taken_from_a_repository() {
        let repo = tempfile::tempdir().unwrap();
        let claim = Claim {
            plugin: "test".into(),
            approve: vec![],
            keys: vec![("directories".into(), bingo_sdk::Merge::PersonalAccumulate)],
        };
        let user = Layer::new(
            "user",
            json!({"directories":["personal"]})
                .as_object()
                .unwrap()
                .clone(),
        )
        .user();
        let project = Layer::new(
            "project",
            json!({"directories":["malicious"]})
                .as_object()
                .unwrap()
                .clone(),
        )
        .project(repo.path())
        .unwrap();
        let cli = Layer::new(
            "cli",
            json!({"directories":["explicit"]})
                .as_object()
                .unwrap()
                .clone(),
        );
        let merged = super::super::merge(&[user, project, cli], &[claim]).unwrap();
        assert_eq!(
            merged.plugins["test"]["directories"],
            json!(["personal", "explicit"])
        );
        assert!(
            merged
                .notices
                .iter()
                .any(|(code, _)| code == "PROJECT_SETTING_IGNORED")
        );
    }

    #[test]
    fn project_cannot_approve_itself_or_forge_waiting_metadata() {
        let repo = tempfile::tempdir().unwrap();
        let root = crate::approval::root_key(repo.path()).unwrap();
        let server = json!({"command":"never-run"});
        let approved = json!({root.clone(): {"mcpServers:p": digest_entry(&server)}});
        let claim = Claim {
            plugin: "test".into(),
            keys: vec![("mcpServers".into(), bingo_sdk::Merge::ByName)],
            approve: vec!["mcpServers".into()],
        };
        let project = Layer::new(
            "project",
            json!({"mcpServers":{"p":server},"approved":approved,"waiting":[]})
                .as_object()
                .unwrap()
                .clone(),
        )
        .project(repo.path())
        .unwrap();
        let merged =
            super::super::merge(std::slice::from_ref(&project), std::slice::from_ref(&claim))
                .unwrap();
        assert!(merged.kernel.approved.is_empty());
        assert_eq!(merged.plugins["test"]["waiting"][0]["name"], "p");
        assert!(
            merged
                .notices
                .iter()
                .any(|(code, _)| code == "APPROVED_IGNORED")
        );
        let user = Layer::new(
            "user",
            json!({"approved":approved}).as_object().unwrap().clone(),
        )
        .user();
        let merged = super::super::merge(&[user, project], &[claim]).unwrap();
        assert_eq!(merged.plugins["test"]["waiting"], json!([]));
    }
}
