//! `<data_dir>/auth.json` (ADR-0012 §2): one entry per provider id.
//!
//! Separate from settings on purpose — the settings project layer is
//! committed, and a token must never be committable. The shape is opencode's,
//! so a credential minted by either tool reads in the other; the path is
//! ours, and neither reads the other's file.

mod binding;
mod signin;
use binding::Binding;
pub use binding::Metadata;

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::AuthError;

const AUTH_FILE: &str = "auth.json";

// Plugins and runtime instances can construct independent handles to the same file.
static STORE_LOCK: Mutex<()> = Mutex::new(());

/// One provider's credential. An OAuth entry is refreshed in place; an API
/// entry is a key minted elsewhere and used as the bearer as it is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Entry {
    #[serde(rename = "oauth")]
    OAuth {
        access: String,
        refresh: String,
        /// Unix seconds; `0` when the issuer said nothing.
        expires: i64,
        #[serde(rename = "accountId", default, skip_serializing_if = "Option::is_none")]
        account_id: Option<String>,
    },
    #[serde(rename = "api")]
    Api { key: String },
    /// One MCP server's sign-in, under the key `mcp:<server name>` (ADR-0050
    /// §2). The registration and the tokens are one fact about one server:
    /// the client bingo registered is only usable against the issuer it was
    /// registered with, and only through the redirect URI it named, so a
    /// discovery that yields a different issuer drops this whole entry.
    #[serde(rename = "mcpOauth", rename_all = "camelCase")]
    McpOAuth {
        issuer: String,
        client_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        client_secret: Option<String>,
        redirect_uri: String,
        access: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        refresh: Option<String>,
        /// Unix seconds; `0` when the issuer said nothing.
        expires: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        scope: Option<String>,
    },
}

/// The store key one MCP server's credential lives under.
pub fn mcp_key(server: &str) -> String {
    format!("mcp:{server}")
}

/// Read-modify-write over the whole file. bingo is one process per user, so
/// an in-process lock is the only lock; the rename is what makes a concurrent
/// reader see either the old file or the new one and never half of either.
#[derive(Clone, Debug)]
pub struct CredentialStore {
    path: PathBuf,
    owner: Option<String>,
    binding: Result<Option<Binding>, String>,
    resource: Result<Option<Binding>, String>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Stored {
    #[serde(flatten)]
    entry: Entry,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    origin: Option<String>,
    #[serde(
        default,
        rename = "resourceOrigin",
        skip_serializing_if = "Option::is_none"
    )]
    resource_origin: Option<String>,
}

/// A refresh can replace only the credential and binding it started with.
pub(crate) struct Snapshot(Stored);

impl Snapshot {
    pub(crate) fn entry(&self) -> &Entry {
        &self.0.entry
    }
}

impl CredentialStore {
    pub(crate) fn snapshot(&self, name: &str) -> Result<Snapshot, AuthError> {
        self.validate_address()?;
        let _guard = self.locked();
        let stored = self.load()?.remove(name).ok_or(AuthError::SignedOut)?;
        self.check_owner(&stored)?;
        self.check_binding(name, &stored)?;
        Ok(Snapshot(stored))
    }

    /// False means the snapshot changed; an error means persistence failed.
    pub(crate) fn replace(
        &self,
        name: &str,
        previous: &Snapshot,
        entry: Option<Entry>,
    ) -> Result<bool, AuthError> {
        let _guard = self.locked();
        let mut entries = self.load()?;
        if entries.get(name) != Some(&previous.0) {
            return Ok(false);
        }
        if let Some(entry) = entry {
            entries.insert(
                name.into(),
                Stored {
                    entry,
                    ..previous.0.clone()
                },
            );
        } else {
            entries.remove(name);
        }
        self.save(&entries)?;
        Ok(true)
    }

    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            path: data_dir.join(AUTH_FILE),
            owner: None,
            binding: Ok(None),
            resource: Ok(None),
        }
    }

    /// A named provider must own an entry before reading or deleting it.
    pub fn owned_by(&self, owner: &str) -> Self {
        Self {
            owner: Some(owner.into()),
            ..self.clone()
        }
    }

    fn check_owner(&self, stored: &Stored) -> Result<(), AuthError> {
        if self.owner.is_some() && self.owner != stored.owner {
            return Err(AuthError::Store(
                "credential belongs to a different or unknown owner".into(),
            ));
        }
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn read(&self, provider: &str) -> Result<Option<Entry>, AuthError> {
        self.validate_address()?;
        let _guard = self.locked();
        self.load()?
            .remove(provider)
            .map(|stored| {
                self.check_owner(&stored)?;
                self.check_binding(provider, &stored)?;
                Ok(stored.entry)
            })
            .transpose()
    }

    pub fn write(&self, provider: &str, entry: Entry) -> Result<(), AuthError> {
        self.validate_address()?;
        let _guard = self.locked();
        let mut entries = self.load()?;
        if let Some(stored) = entries.get(provider) {
            self.check_owner(stored)?;
        }
        entries.insert(
            provider.to_string(),
            Stored {
                entry,
                owner: self.owner.clone(),
                origin: self.origin(),
                resource_origin: self.resource_origin(),
            },
        );
        self.save(&entries)
    }

    /// A provider that was never there is already removed.
    pub fn remove(&self, provider: &str) -> Result<(), AuthError> {
        let _guard = self.locked();
        let mut entries = self.load()?;
        match entries.remove(provider) {
            Some(stored) => {
                self.check_owner(&stored)?;
                self.save(&entries)
            }
            None => Ok(()),
        }
    }

    /// A poisoned lock still guards a consistent file: every write is a whole
    /// file, so a panic mid-write left nothing half-applied to protect.
    fn locked(&self) -> std::sync::MutexGuard<'_, ()> {
        STORE_LOCK
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    /// A missing file is an empty store; a corrupt one is an error, never
    /// silently emptied — emptying it would delete a credential to hide a bug.
    fn load(&self) -> Result<BTreeMap<String, Stored>, AuthError> {
        let raw = match std::fs::read_to_string(&self.path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(BTreeMap::new());
            }
            Err(_) => {
                return Err(AuthError::Store(format!(
                    "could not read {}",
                    self.display()
                )));
            }
        };
        serde_json::from_str(&raw)
            .map_err(|_| AuthError::Store(format!("could not read {}", self.display())))
    }

    fn save(&self, entries: &BTreeMap<String, Stored>) -> Result<(), AuthError> {
        let directory = self
            .path
            .parent()
            .ok_or_else(|| AuthError::Store(format!("{} has no directory", self.display())))?;
        std::fs::create_dir_all(directory).map_err(|e| self.io("create the directory", e))?;
        let temporary = directory.join(format!("{AUTH_FILE}.tmp"));
        self.write_private(&temporary, entries)?;
        std::fs::rename(&temporary, &self.path).map_err(|e| self.io("rename", e))
    }

    /// The mode is set on the empty file, before a byte of it exists: a
    /// credential is never readable by anyone else, not even for an instant.
    fn write_private(
        &self,
        temporary: &Path,
        entries: &BTreeMap<String, Stored>,
    ) -> Result<(), AuthError> {
        let mut file = std::fs::File::create(temporary).map_err(|e| self.io("create", e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(std::fs::Permissions::from_mode(0o600))
                .map_err(|e| self.io("restrict", e))?;
        }
        let json = serde_json::to_string_pretty(entries)
            .map_err(|e| AuthError::Store(format!("encode: {e}")))?;
        file.write_all(json.as_bytes())
            .map_err(|e| self.io("write", e))?;
        file.sync_all().map_err(|e| self.io("flush", e))
    }

    fn io(&self, what: &str, error: std::io::Error) -> AuthError {
        AuthError::Store(format!("{what} {}: {error}", self.display()))
    }

    fn display(&self) -> std::path::Display<'_> {
        self.path.display()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn store(directory: &tempfile::TempDir) -> CredentialStore {
        CredentialStore::new(directory.path().join("data"))
    }

    fn oauth() -> Entry {
        Entry::OAuth {
            access: "access-1".into(),
            refresh: "refresh-1".into(),
            expires: 1_786_000_000,
            account_id: Some("acc_1".into()),
        }
    }

    #[test]
    fn named_credentials_keep_their_owner_across_handles() {
        let directory = tempfile::tempdir().unwrap();
        let store = store(&directory);
        let owner = store.owned_by("family-a");
        owner
            .write("work", Entry::Api { key: "test".into() })
            .unwrap();
        assert!(store.owned_by("family-a").read("work").unwrap().is_some());
        let foreign = store.owned_by("family-b");
        assert!(foreign.read("work").is_err());
        assert!(foreign.remove("work").is_err());
        assert!(
            foreign
                .write(
                    "work",
                    Entry::Api {
                        key: "other".into()
                    }
                )
                .is_err()
        );
        assert!(owner.read("work").unwrap().is_some());
        let raw: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(store.path()).unwrap()).unwrap();
        assert_eq!(raw["work"]["owner"], "family-a");
    }

    #[test]
    fn independent_handles_do_not_overwrite_concurrent_entries() {
        let directory = tempfile::tempdir().unwrap();
        let barrier = std::sync::Barrier::new(8);
        std::thread::scope(|threads| {
            for n in 0..8 {
                let path = directory.path().to_path_buf();
                let barrier = &barrier;
                threads.spawn(move || {
                    let store = CredentialStore::new(path);
                    barrier.wait();
                    store
                        .write(
                            &format!("provider-{n}"),
                            Entry::Api {
                                key: format!("key-{n}"),
                            },
                        )
                        .unwrap();
                });
            }
        });
        let store = CredentialStore::new(directory.path().to_path_buf());
        for n in 0..8 {
            assert_eq!(
                store.read(&format!("provider-{n}")).unwrap(),
                Some(Entry::Api {
                    key: format!("key-{n}")
                })
            );
        }
    }

    #[test]
    fn a_missing_file_reads_as_an_empty_store() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        assert_eq!(store(&directory).read("codex").expect("a read"), None);
    }

    #[test]
    fn an_entry_round_trips_and_its_neighbours_are_untouched() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let store = store(&directory);
        store.write("codex", oauth()).expect("a write");
        store
            .write("other", Entry::Api { key: "sk-1".into() })
            .expect("a write");
        assert_eq!(store.read("codex").expect("a read"), Some(oauth()));

        // A second store over the same directory is a later process.
        let reopened = CredentialStore::new(directory.path().join("data"));
        assert_eq!(reopened.read("codex").expect("a read"), Some(oauth()));
        reopened.remove("codex").expect("a removal");
        assert_eq!(reopened.read("codex").expect("a read"), None);
        assert_eq!(
            reopened.read("other").expect("a read"),
            Some(Entry::Api { key: "sk-1".into() })
        );
        reopened.remove("codex").expect("removing twice is a no-op");
    }

    #[cfg(unix)]
    #[test]
    fn the_file_is_private_after_the_first_write_and_after_a_rewrite() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().expect("a temporary directory");
        let store = store(&directory);
        let mode = || {
            std::fs::metadata(store.path())
                .expect("the file")
                .permissions()
                .mode()
                & 0o777
        };
        store.write("codex", oauth()).expect("a write");
        assert_eq!(mode(), 0o600, "0600 after the first write");
        store
            .write("codex", Entry::Api { key: "sk-2".into() })
            .expect("a rewrite");
        assert_eq!(mode(), 0o600, "still 0600 after a rewrite");
    }

    #[test]
    fn a_corrupt_file_is_an_error_rather_than_a_silent_emptying() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let store = store(&directory);
        std::fs::create_dir_all(directory.path().join("data")).expect("the directory");
        std::fs::write(store.path(), "{not json").expect("a corrupt file");
        assert!(matches!(store.read("codex"), Err(AuthError::Store(_))));
    }

    /// The shape ADR-0012 §2 names, compared as JSON so a field rename fails
    /// here rather than against a live issuer.
    #[test]
    fn the_serialised_shape_is_the_one_the_adr_names() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let store = store(&directory);
        store.write("codex", oauth()).expect("a write");
        store
            .write("openai", Entry::Api { key: "sk-1".into() })
            .expect("a write");
        let raw = std::fs::read_to_string(store.path()).expect("the file");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&raw).expect("json"),
            json!({
                "codex": {
                    "type": "oauth",
                    "access": "access-1",
                    "refresh": "refresh-1",
                    "expires": 1_786_000_000,
                    "accountId": "acc_1",
                },
                "openai": { "type": "api", "key": "sk-1" },
            })
        );
    }

    /// ADR-0050 §2's shape, pinned as JSON: this file is read back by later
    /// runs and by a person, so a renamed field fails here and not against a
    /// server that then asks for a fresh sign-in nobody understands.
    #[test]
    fn an_mcp_entry_is_written_under_its_server_in_the_shape_the_adr_names() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let store = store(&directory);
        store
            .write(
                &mcp_key("binlesson"),
                Entry::McpOAuth {
                    issuer: "https://as.example.com".into(),
                    client_id: "cl_1".into(),
                    client_secret: None,
                    redirect_uri: "http://localhost:1455/auth/callback".into(),
                    access: "at_1".into(),
                    refresh: Some("rt_1".into()),
                    expires: 1_786_000_000,
                    scope: Some("mcp:tools".into()),
                },
            )
            .expect("a write");
        let raw = std::fs::read_to_string(store.path()).expect("the file");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&raw).expect("json"),
            json!({
                "mcp:binlesson": {
                    "type": "mcpOauth",
                    "issuer": "https://as.example.com",
                    "clientId": "cl_1",
                    "redirectUri": "http://localhost:1455/auth/callback",
                    "access": "at_1",
                    "refresh": "rt_1",
                    "expires": 1_786_000_000,
                    "scope": "mcp:tools",
                }
            })
        );
    }

    /// A public client has no secret, an issuer may grant no refresh token
    /// and may name no scope: none of the three is written as a null.
    #[test]
    fn what_the_server_never_gave_is_absent_rather_than_empty() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let store = store(&directory);
        let entry = Entry::McpOAuth {
            issuer: "https://as.example.com".into(),
            client_id: "cl_1".into(),
            client_secret: None,
            redirect_uri: "http://127.0.0.1:1455/auth/callback".into(),
            access: "at_1".into(),
            refresh: None,
            expires: 0,
            scope: None,
        };
        store
            .write(&mcp_key("plain"), entry.clone())
            .expect("write");
        let raw = std::fs::read_to_string(store.path()).expect("the file");
        for absent in ["clientSecret", "refresh", "scope"] {
            assert!(!raw.contains(absent), "{absent} is in {raw}");
        }
        assert_eq!(store.read(&mcp_key("plain")).expect("a read"), Some(entry));
    }

    #[test]
    fn an_account_id_nobody_knows_is_left_out_of_the_file() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let store = store(&directory);
        store
            .write(
                "codex",
                Entry::OAuth {
                    access: "a".into(),
                    refresh: "r".into(),
                    expires: 0,
                    account_id: None,
                },
            )
            .expect("a write");
        let raw = std::fs::read_to_string(store.path()).expect("the file");
        assert!(!raw.contains("accountId"), "{raw}");
    }
}
