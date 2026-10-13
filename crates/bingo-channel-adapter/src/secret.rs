//! Where a channel's secret comes from (ADR-0020 §8).
//!
//! Until now it was the environment and nothing else. That is right for a
//! bingo started from a shell and impossible for one started at boot: launchd
//! and systemd hand a service no exported variables, and writing the secret
//! into the service file would put it in a file that is neither 0600 nor
//! private — the very thing ADR-0012 §2 moved credentials out of settings to
//! avoid.
//!
//! So there are two sources and a fixed order. The environment still wins, and
//! a shell that exports the variable behaves exactly as it did; `auth.json`
//! answers when the shell has nothing to say. Nothing here ever renders the
//! secret: a [`Source`] is what a person is shown, and it names a variable or
//! a file.

use std::path::PathBuf;

use bingo_auth_oauth::{CredentialStore, Entry};
use bingo_sdk::Env;

/// `auth.json`'s key for one adapter's secret. One namespace, so a channel can
/// never collide with a provider's entry.
pub fn credential(id: &str) -> String {
    format!("channels.{id}")
}

/// Where a secret came from. Never the secret itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// An exported variable, which wins whenever it is set.
    Environment { variable: String },
    /// `auth.json`, under [`credential`].
    Store { path: PathBuf, key: String },
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Source::Environment { variable } => write!(f, "the environment ({variable})"),
            Source::Store { path, key } => write!(f, "{} ({key})", path.display()),
        }
    }
}

/// A secret and where it was found.
#[derive(Clone, Debug)]
pub struct Found {
    pub source: Source,
    pub secret: String,
}

/// The secret an adapter signs with: the environment, else the store, else
/// nothing at all.
///
/// A variable exported as the empty string counts as nothing: it is what a
/// half-written shell profile leaves behind, and treating it as a secret only
/// turns a missing credential into an authentication error further away.
pub fn find(env: &Env, id: &str, variable: &str) -> Option<Found> {
    let exported = std::env::var(variable).ok().filter(|set| !set.is_empty());
    if let Some(secret) = exported {
        return Some(Found {
            source: Source::Environment {
                variable: variable.to_string(),
            },
            secret,
        });
    }
    let key = credential(id);
    let store = CredentialStore::new(env.data_dir.clone());
    let Some(Entry::Api { key: secret }) = store.read(&key).ok().flatten() else {
        return None;
    };
    Some(Found {
        source: Source::Store {
            path: store.path().to_path_buf(),
            key,
        },
        secret,
    })
}

/// Write one adapter's secret to the store, and say where it went.
pub fn store(env: &Env, id: &str, secret: String) -> Result<PathBuf, String> {
    let store = CredentialStore::new(env.data_dir.clone());
    store
        .write(&credential(id), Entry::Api { key: secret })
        .map_err(|e| e.to_string())?;
    Ok(store.path().to_path_buf())
}

/// One adapter these settings name, and what it signs with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Requirement {
    /// The adapter id — what `bingo channels secret <id>` takes.
    pub id: &'static str,
    /// The variable it reads, where it needs a secret at all.
    pub variable: Option<&'static str>,
}

/// The platforms for which a secret can be stored.
pub fn signing(platforms: &crate::Platforms) -> Vec<Requirement> {
    platforms
        .iter()
        .filter_map(|platform| platform.secret())
        .collect()
}

/// Validate every named platform before reporting its credentials.
pub fn configured(
    settings: &serde_json::Value,
    platforms: &crate::Platforms,
) -> Result<Vec<Requirement>, String> {
    match settings.get("channels") {
        None => Ok(Vec::new()),
        Some(serde_json::Value::Object(slices)) => crate::platform::configured(slices, platforms),
        Some(_) => Err("channels must be an object".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn env(home: &std::path::Path) -> Env {
        Env::rooted(home)
    }

    #[test]
    fn a_secret_is_the_store_s_when_the_environment_has_none() {
        let home = tempfile::tempdir().expect("a temporary home");
        let env = env(home.path());
        assert!(
            find(&env, "sample", "BINGO_TEST_SECRET_NEVER_SET").is_none(),
            "neither source has one"
        );

        let path = store(&env, "sample", "s-from-disk".into()).expect("it is written");
        let found = find(&env, "sample", "BINGO_TEST_SECRET_NEVER_SET").expect("the store has it");
        assert_eq!(found.secret, "s-from-disk");
        assert_eq!(
            found.source,
            Source::Store {
                path,
                key: "channels.sample".into()
            }
        );
        assert!(
            found.source.to_string().contains("channels.sample"),
            "what a person is shown names the key, not the secret: {}",
            found.source
        );
        assert!(!found.source.to_string().contains("s-from-disk"));
    }

    #[test]
    fn the_store_is_keyed_so_a_channel_can_never_collide_with_a_provider() {
        assert_eq!(credential("sample"), "channels.sample");
        assert_eq!(credential("loopback"), "channels.loopback");
    }
    #[test]
    fn only_named_platforms_are_configured_and_only_those_with_secrets_sign() {
        use serde_json::json;
        use std::sync::Arc;
        let platforms: crate::Platforms = vec![
            Arc::new(crate::testing::TestPlatform),
            Arc::new(crate::loopback::Platform),
        ];
        assert!(configured(&json!({}), &platforms).unwrap().is_empty());
        assert_eq!(
            configured(&json!({"channels": {"loopback": {}}}), &platforms).unwrap(),
            [Requirement {
                id: "loopback",
                variable: None
            }]
        );
        assert_eq!(
            configured(&json!({"channels": {"sample": {}}}), &platforms).unwrap(),
            signing(&platforms)
        );
        assert_eq!(
            configured(
                &json!({"channels": {"sample": {}, "loopback": {}}}),
                &platforms
            )
            .unwrap()
            .len(),
            2
        );
        assert!(configured(&json!({"channels": "nonsense"}), &platforms).is_err());
    }
}
