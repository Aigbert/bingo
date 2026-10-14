//! The user layer is the sole authority for repository code.
mod digest;
mod entry;
use crate::settings;
use bingo_sdk::{Approval, ApprovalReader, Env, ErrorCode, KernelError, PluginState};
pub use digest::{digest_dir, digest_entry};
use std::{
    collections::BTreeMap,
    path::{Component, Path},
    sync::Mutex,
};

pub type Approved = BTreeMap<String, BTreeMap<String, String>>;

pub fn verdict(approved: &Approved, root: &str, what: &str, digest: &str) -> PluginState {
    match approved.get(root).and_then(|entries| entries.get(what)) {
        Some(saved) if saved == digest => PluginState::On,
        saved => PluginState::Waiting {
            changed: saved.is_some(),
        },
    }
}
pub fn root_key(root: &Path) -> Result<String, KernelError> {
    let root = root.canonicalize().map_err(failed)?;
    root.to_str()
        .map(str::to_string)
        .ok_or_else(|| failed("non-UTF-8 repository paths cannot be approved"))
}
fn failed(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::InvalidInput, error.to_string())
}
pub fn plugin_dir(request: &Approval) -> Result<std::path::PathBuf, KernelError> {
    let name = request
        .what
        .strip_prefix("plugin:")
        .ok_or_else(|| failed("expected plugin:<name>"))?;
    if name.is_empty()
        || name.contains(['/', '\\', ':'])
        || !matches!(
            Path::new(name).components().collect::<Vec<_>>().as_slice(),
            [Component::Normal(_)]
        )
    {
        return Err(failed("approval requires one plugin name"));
    }
    let root = request.root.canonicalize().map_err(failed)?;
    let mut dir = root;
    for component in [".bingo", "plugins", name] {
        dir.push(component);
        if std::fs::symlink_metadata(&dir)
            .map_err(failed)?
            .file_type()
            .is_symlink()
        {
            return Err(failed("plugin directories cannot be symlinks"));
        }
    }
    Ok(dir)
}

pub struct Authority {
    env: Env,
    approved: Mutex<Approved>,
    entries: BTreeMap<(String, String), String>,
}
impl Authority {
    pub fn new(env: Env, approved: Approved, entries: BTreeMap<(String, String), String>) -> Self {
        Self {
            env,
            approved: Mutex::new(approved),
            entries,
        }
    }
    fn digest(&self, request: &Approval) -> Result<String, KernelError> {
        if request.what.starts_with("plugin:") {
            let dir = plugin_dir(request)?;
            let digest = digest_dir(&dir).map_err(failed)?;
            digest::contained(&dir).map_err(failed)?;
            entry::contained(&dir).map_err(|error| failed(format!("{}: {error}", request.what)))?;
            return Ok(digest);
        }
        let key = (root_key(&request.root)?, request.what.clone());
        self.entries
            .get(&key)
            .cloned()
            .ok_or_else(|| failed("no repository entry with that name"))
    }
    pub fn approve(&self, request: &Approval) -> Result<String, KernelError> {
        let digest = self.digest(request)?;
        let root = root_key(&request.root)?;
        let mut held = self
            .approved
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let path = self.env.user_settings();
        let mut document = settings::read_document(&path).map_err(failed)?;
        let mut approved: Approved = document
            .get("approved")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(failed)?
            .unwrap_or_default();
        approved
            .entry(root)
            .or_default()
            .insert(request.what.clone(), digest.clone());
        document.insert(
            "approved".into(),
            serde_json::to_value(&approved).map_err(failed)?,
        );
        settings::write(&path, &document).map_err(failed)?;
        *held = approved;
        Ok(digest)
    }
}
impl ApprovalReader for Authority {
    fn state(&self, request: &Approval) -> Result<PluginState, KernelError> {
        let root = root_key(&request.root)?;
        let digest = self.digest(request)?;
        let held = self
            .approved
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        Ok(verdict(&held, &root, &request.what, &digest))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn approval_and_spawn_verdict_refuse_entry_paths_outside_the_digest() {
        let home = tempfile::tempdir().unwrap();
        let repo = tempfile::tempdir().unwrap();
        let dir = repo.path().join(".bingo/plugins/p");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(repo.path().join("outside.js"), "original").unwrap();
        std::fs::write(dir.join("inside.js"), "inside").unwrap();
        let authority = Authority::new(Env::rooted(home.path()), Approved::new(), BTreeMap::new());
        let request = Approval {
            root: repo.path().into(),
            what: "plugin:p".into(),
        };
        for (command, args) in [
            ("${PLUGIN_ROOT}/../../../outside.js", vec![]),
            ("node", vec!["${PLUGIN_ROOT}/../../../outside.js"]),
            ("node", vec!["../../../outside.js"]),
            ("node", vec!["..\\..\\..\\outside.js"]),
        ] {
            let manifest = serde_json::json!({"entry": {"command":command, "args":args}});
            std::fs::write(dir.join("plugin.json"), manifest.to_string()).unwrap();
            let error = authority
                .approve(&request)
                .expect_err("outside bytes cannot be approved");
            assert!(error.message.contains("plugin:p"), "{error}");
            assert!(authority.state(&request).is_err(), "spawn must refuse too");
        }
        std::fs::write(
            dir.join("plugin.json"),
            r#"{"entry":{"command":"node","args":["${PLUGIN_ROOT}/inside.js"]}}"#,
        )
        .unwrap();
        authority.approve(&request).unwrap();
        assert_eq!(authority.state(&request).unwrap(), PluginState::On);
    }

    #[test]
    fn a_yes_names_root_item_and_bytes() {
        let approved = BTreeMap::from([(
            "repo".into(),
            BTreeMap::from([("plugin:p".into(), "sha256:a".into())]),
        )]);
        for (root, what, digest, expected) in [
            ("repo", "plugin:p", "sha256:a", PluginState::On),
            (
                "repo",
                "plugin:p",
                "sha256:b",
                PluginState::Waiting { changed: true },
            ),
            (
                "elsewhere",
                "plugin:p",
                "sha256:a",
                PluginState::Waiting { changed: false },
            ),
            (
                "repo",
                "mcpServers:p",
                "sha256:a",
                PluginState::Waiting { changed: false },
            ),
        ] {
            assert_eq!(verdict(&approved, root, what, digest), expected);
        }
    }
    #[test]
    fn approval_keeps_comments_and_rejects_path_traversal() {
        let home = tempfile::tempdir().unwrap();
        let repo = tempfile::tempdir().unwrap();
        let env = Env::rooted(home.path());
        std::fs::create_dir_all(&env.config_dir).unwrap();
        std::fs::write(env.user_settings(), "# keep me\nmodel = 'a' # also me\n").unwrap();
        let dir = repo.path().join(".bingo/plugins/p");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plugin.json"), "{}").unwrap();
        let authority = Authority::new(env.clone(), Approved::new(), BTreeMap::new());
        let mut request = Approval {
            root: repo.path().into(),
            what: "plugin:p".into(),
        };
        authority.approve(&request).unwrap();
        assert_eq!(authority.state(&request).unwrap(), PluginState::On);
        let text = std::fs::read_to_string(env.user_settings()).unwrap();
        assert!(text.contains("# keep me") && text.contains("# also me"));
        std::fs::write(dir.join("plugin.json"), "[]").unwrap();
        assert_eq!(
            authority.state(&request).unwrap(),
            PluginState::Waiting { changed: true }
        );
        for name in ["../p", "..\\p", "", "/p", "C:p"] {
            request.what = format!("plugin:{name}");
            assert!(authority.approve(&request).is_err());
        }
    }
}
