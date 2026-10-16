//! Where a plugin lives, and what reading one directory finds.
//!
//! Repository candidates wait before they can take a user plugin's name.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bingo_sdk::Env;

use crate::manifest::Manifest;
use crate::notice::{Notice, Notices};

/// The directory a layer keeps its plugins in.
const PLUGINS: &str = "plugins";

/// The directory a project keeps its own configuration in.
const PROJECT: &str = ".bingo";

/// The file a plugin directory is known by.
pub const MANIFEST_FILE: &str = "plugin.json";

/// One plugin directory, read.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    /// The directory's name, which is the plugin's name.
    pub name: String,
    pub root: PathBuf,
    pub origin: bingo_sdk::PluginOrigin,
    pub manifest: Manifest,
    pub state: bingo_sdk::PluginState,
}

/// Every directory a plugin may live in, least important first: a later layer
/// overrides a name an earlier one used.
pub fn dirs(env: &Env, cwd: &Path) -> Vec<PathBuf> {
    let config = env.config_dir.join(PLUGINS);
    let project = cwd.join(PROJECT).join(PLUGINS);
    if config == project {
        return vec![config];
    }
    vec![config, project]
}

pub fn approved(
    dirs: &[PathBuf],
    explicit: &[PathBuf],
    notices: &Notices,
    authority: Option<&dyn bingo_sdk::ApprovalReader>,
) -> Vec<Found> {
    let mut found = BTreeMap::new();
    let mut waiting = Vec::new();
    for (index, dir) in dirs.iter().enumerate() {
        for mut plugin in layer(dir, notices) {
            plugin.origin = if index == 0 {
                bingo_sdk::PluginOrigin::User
            } else {
                bingo_sdk::PluginOrigin::Project
            };
            if index != 0 {
                plugin.state = project_state(&plugin, authority);
                if let bingo_sdk::PluginState::Failed { why } = &plugin.state {
                    notices.push(Notice::warn(
                        "PLUGIN_REFUSED",
                        format!("{}: {why}", plugin.name),
                    ));
                }
                if !plugin.state.is_on() {
                    waiting.push(plugin);
                    continue;
                }
            }
            found.insert(plugin.name.clone(), plugin);
        }
    }
    for root in explicit {
        if let Some(plugin) = read(root, notices) {
            found.insert(plugin.name.clone(), plugin);
        }
    }
    let mut found: Vec<_> = found.into_values().collect();
    found.extend(waiting);
    found
}

pub fn request(found: &Found) -> Option<bingo_sdk::Approval> {
    Some(bingo_sdk::Approval {
        root: found.root.parent()?.parent()?.parent()?.to_path_buf(),
        what: format!("plugin:{}", found.name),
    })
}
pub fn project_state(
    found: &Found,
    authority: Option<&dyn bingo_sdk::ApprovalReader>,
) -> bingo_sdk::PluginState {
    let Some((authority, request)) = authority.zip(request(found)) else {
        return bingo_sdk::PluginState::Waiting { changed: false };
    };
    authority
        .state(&request)
        .unwrap_or_else(|error| bingo_sdk::PluginState::Failed { why: error.message })
}

fn layer(dir: &Path, notices: &Notices) -> Vec<Found> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        if let Some(plugin) = read(&entry.path(), notices) {
            found.push(plugin);
        }
    }
    found
}

/// One directory, if it holds a manifest that names itself. A directory with
/// no `plugin.json` is somebody else's; a `plugin.json` that will not parse is
/// a plugin a person meant to have, so that one is reported.
pub fn read(root: &Path, notices: &Notices) -> Option<Found> {
    let name = root.file_name()?.to_str()?.to_string();
    let raw = std::fs::read_to_string(root.join(MANIFEST_FILE)).ok()?;
    match serde_json::from_str::<Manifest>(&raw) {
        Ok(manifest) if manifest.name == name => Some(Found {
            name,
            root: root.to_path_buf(),
            origin: bingo_sdk::PluginOrigin::Explicit,
            manifest,
            state: bingo_sdk::PluginState::On,
        }),
        Ok(manifest) => {
            notices.push(Notice::warn(
                "PLUGIN_MISNAMED",
                format!(
                    "{}/{MANIFEST_FILE} calls itself `{}`; a plugin is named by its directory",
                    root.display(),
                    manifest.name
                ),
            ));
            None
        }
        Err(error) => {
            notices.push(Notice::warn(
                "PLUGIN_UNREADABLE",
                format!("{}/{MANIFEST_FILE}: {error}", root.display()),
            ));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn manifest(name: &str) -> String {
        json!({
            "name": name,
            "version": "0.1.0",
            "entry": { "command": "python3", "args": ["${PLUGIN_ROOT}/main.py"] }
        })
        .to_string()
    }

    fn plugin(dir: &Path, name: &str, body: &str) {
        let root = dir.join(name);
        std::fs::create_dir_all(&root).expect("a plugin directory");
        std::fs::write(root.join(MANIFEST_FILE), body).expect("a manifest");
    }

    #[test]
    fn a_refused_entry_is_named_in_a_notice_and_shadows_nothing() {
        struct Refused;
        impl bingo_sdk::ApprovalReader for Refused {
            fn state(
                &self,
                _: &bingo_sdk::Approval,
            ) -> Result<bingo_sdk::PluginState, bingo_sdk::KernelError> {
                Err(bingo_sdk::KernelError::new(
                    bingo_sdk::ErrorCode::InvalidInput,
                    "entry path escapes",
                ))
            }
        }
        let home = tempfile::tempdir().unwrap();
        let repo = tempfile::tempdir().unwrap();
        for dir in [&home, &repo] {
            plugin(dir.path(), "p", &manifest("p"));
        }
        let notices = Notices::default();
        let found = approved(
            &[home.path().into(), repo.path().into()],
            &[],
            &notices,
            Some(&Refused),
        );
        assert_eq!(found[0].root, home.path().join("p"));
        assert!(matches!(
            found[1].state,
            bingo_sdk::PluginState::Failed { .. }
        ));
        let said = notices.drain();
        assert_eq!(said.len(), 1);
        assert!(said[0].text.contains("p: entry path escapes"));
    }

    #[test]
    fn explicit_directories_win_by_name_in_argument_order() {
        let home = tempfile::tempdir().unwrap();
        let first = tempfile::tempdir().unwrap();
        let last = tempfile::tempdir().unwrap();
        for dir in [&home, &first, &last] {
            plugin(dir.path(), "wordcount", &manifest("wordcount"));
        }
        let found = approved(
            &[home.path().into()],
            &[
                first.path().join("wordcount"),
                last.path().join("wordcount"),
            ],
            &Notices::default(),
            None,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].root, last.path().join("wordcount"));
    }

    #[test]
    fn the_person_s_own_plugins_come_before_the_project_s() {
        let env = Env::rooted("/home/user");
        assert_eq!(
            dirs(&env, Path::new("/work/repo")),
            [
                PathBuf::from("/home/user/.bingo/plugins"),
                PathBuf::from("/work/repo/.bingo/plugins"),
            ]
        );
    }

    #[test]
    fn a_home_that_is_the_working_directory_is_read_once() {
        let env = Env::rooted("/work");
        assert_eq!(
            dirs(&env, Path::new("/work")),
            [PathBuf::from("/work/.bingo/plugins")]
        );
    }

    #[test]
    fn a_layer_that_is_not_there_finds_nothing_and_says_nothing() {
        let notices = Notices::default();
        assert!(approved(&[PathBuf::from("/no/such/dir")], &[], &notices, None).is_empty());
        assert!(notices.drain().is_empty());
    }

    #[test]
    fn a_waiting_project_keeps_the_persons_namesake() {
        let config = tempfile::tempdir().expect("a config directory");
        let project = tempfile::tempdir().expect("a project directory");
        plugin(config.path(), "wordcount", &manifest("wordcount"));
        plugin(project.path(), "wordcount", &manifest("wordcount"));
        let notices = Notices::default();
        let found = approved(
            &[config.path().to_path_buf(), project.path().to_path_buf()],
            &[],
            &notices,
            None,
        );
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].root, config.path().join("wordcount"));
        assert_eq!(found[1].root, project.path().join("wordcount"));
        assert_eq!(
            found[1].state,
            bingo_sdk::PluginState::Waiting { changed: false }
        );
    }

    #[test]
    fn a_directory_with_no_manifest_is_somebody_else_s() {
        let dir = tempfile::tempdir().expect("a directory");
        std::fs::create_dir_all(dir.path().join("notes")).expect("a plain directory");
        let notices = Notices::default();
        assert!(approved(&[dir.path().to_path_buf()], &[], &notices, None).is_empty());
        assert!(notices.drain().is_empty());
    }

    #[test]
    fn a_manifest_that_will_not_parse_is_reported_not_skipped() {
        let dir = tempfile::tempdir().expect("a directory");
        plugin(dir.path(), "broken", "{ this is not json");
        let notices = Notices::default();
        assert!(approved(&[dir.path().to_path_buf()], &[], &notices, None).is_empty());
        let said = notices.drain();
        assert_eq!(said.len(), 1);
        assert_eq!(said[0].code, "PLUGIN_UNREADABLE");
    }

    #[test]
    fn a_manifest_that_names_another_plugin_is_refused() {
        let dir = tempfile::tempdir().expect("a directory");
        plugin(dir.path(), "wordcount", &manifest("linecount"));
        let notices = Notices::default();
        assert!(approved(&[dir.path().to_path_buf()], &[], &notices, None).is_empty());
        let said = notices.drain();
        assert_eq!(said.len(), 1);
        assert_eq!(said[0].code, "PLUGIN_MISNAMED");
        assert!(said[0].text.contains("linecount"), "{}", said[0].text);
    }

    #[test]
    fn what_is_found_is_the_directory_and_the_manifest_in_it() {
        let dir = tempfile::tempdir().expect("a directory");
        plugin(dir.path(), "wordcount", &manifest("wordcount"));
        let found = approved(&[dir.path().to_path_buf()], &[], &Notices::default(), None);
        assert_eq!(found[0].name, "wordcount");
        assert_eq!(found[0].root, dir.path().join("wordcount"));
        assert_eq!(found[0].manifest.entry().unwrap().command, "python3");
    }
}
