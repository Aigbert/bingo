//! A refresh may only replace the credential and binding it started with.
use super::{CredentialStore, Entry, Stored};
use crate::AuthError;

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

    pub(crate) fn replace(
        &self,
        name: &str,
        previous: &Snapshot,
        entry: Option<Entry>,
    ) -> Result<(), AuthError> {
        let _guard = self.locked();
        let mut entries = self.load()?;
        if entries.get(name) != Some(&previous.0) {
            return Err(AuthError::Binding(format!(
                "{name}: the credential changed during refresh; choose it again or sign in."
            )));
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
        self.save(&entries)
    }
}
