//! Non-secret ownership and origin metadata, checked before exposing an entry.
use super::{CredentialStore, Stored};
use crate::AuthError;

#[derive(Clone, Debug, Default)]
pub struct Metadata {
    pub owner: Option<String>,
    pub origin: Option<String>,
    pub resource_origin: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct Binding {
    pub(super) origin: String,
    pub(super) file: String,
    pub(super) repository: bool,
}

impl Binding {
    fn new(address: &str, file: &str) -> Result<Self, String> {
        let origin = crate::lifetime::origin(address).ok_or_else(|| {
            format!("Invalid credential address in {file}; fix it before signing in.")
        })?;
        Ok(Self {
            origin,
            file: file.into(),
            repository: false,
        })
    }

    fn check(&self, name: &str, stored: Option<&str>) -> Result<(), AuthError> {
        if stored == Some(self.origin.as_str()) {
            return Ok(());
        }
        Err(AuthError::Binding(format!(
            "{name}: the address {} is not the one its key was entered for; sign in again or remove the override in {}",
            self.origin, self.file
        )))
    }
}

fn rebound(
    previous: &Result<Option<Binding>, String>,
    address: &str,
    file: &str,
) -> Result<Option<Binding>, String> {
    let mut binding = Binding::new(address, file)?;
    if let Ok(Some(previous)) = previous
        && previous.origin == binding.origin
    {
        binding.repository = previous.repository;
        binding.file = previous.file.clone();
    }
    Ok(Some(binding))
}

impl CredentialStore {
    pub fn with_repository_origin(mut self, repository: bool) -> Self {
        if let Ok(Some(binding)) = &mut self.binding {
            binding.repository = repository;
        }
        self
    }

    pub fn with_repository_resource(mut self, repository: bool) -> Self {
        if let Ok(Some(binding)) = &mut self.resource {
            binding.repository = repository;
        }
        self
    }

    pub fn check_configured(&self, name: &str, address: &str) -> Result<(), AuthError> {
        self.validate_address()?;
        if let Ok(Some(binding)) = &self.binding {
            binding.check(name, crate::lifetime::origin(address).as_deref())?;
        }
        Ok(())
    }

    pub fn validate_address(&self) -> Result<(), AuthError> {
        for binding in [&self.binding, &self.resource] {
            if let Err(message) = binding {
                return Err(AuthError::Binding(message.clone()));
            }
        }
        Ok(())
    }

    pub fn origin_file(&self) -> Option<&str> {
        self.binding
            .as_ref()
            .ok()
            .and_then(Option::as_ref)
            .map(|binding| binding.file.as_str())
    }

    pub fn resource_file(&self) -> Option<&str> {
        self.resource
            .as_ref()
            .ok()
            .and_then(Option::as_ref)
            .map(|binding| binding.file.as_str())
    }

    pub fn at_origin(&self, address: &str, file: &str) -> Self {
        Self {
            binding: rebound(&self.binding, address, file),
            ..self.clone()
        }
    }

    /// OAuth also sends access tokens to a resource server, separate from its issuer.
    pub fn at_resource(&self, address: &str, file: &str) -> Self {
        Self {
            resource: rebound(&self.resource, address, file),
            ..self.clone()
        }
    }

    pub fn remove_if_owner(&self, name: &str, owner: Option<&str>) -> Result<(), AuthError> {
        let _guard = self.locked();
        let mut entries = self.load()?;
        if let Some(stored) = entries.get(name) {
            if stored.owner.as_deref() != owner {
                return Err(AuthError::Store("credential ownership changed".into()));
            }
            entries.remove(name);
            self.save(&entries)?;
        }
        Ok(())
    }

    pub fn metadata(&self, name: &str) -> Result<Option<Metadata>, AuthError> {
        let _guard = self.locked();
        Ok(self.load()?.remove(name).map(|stored| Metadata {
            owner: stored.owner,
            origin: stored.origin,
            resource_origin: stored.resource_origin,
        }))
    }

    /// Call only with an unambiguous user-layer declaration and its own addresses.
    pub fn adopt(&self, name: &str) -> Result<(), AuthError> {
        self.validate_address()?;
        let _guard = self.locked();
        let mut entries = self.load()?;
        let Some(stored) = entries.get_mut(name) else {
            return Ok(());
        };
        if stored.owner.is_some() {
            self.check_owner(stored)?;
        }
        if stored.owner.is_some()
            && stored.origin.is_some()
            && (self.resource.as_ref().is_ok_and(Option::is_none)
                || stored.resource_origin.is_some())
        {
            return Ok(());
        }
        stored.owner = stored.owner.clone().or(self.owner.clone());
        stored.origin = stored.origin.clone().or_else(|| {
            self.binding
                .as_ref()
                .ok()
                .and_then(Option::as_ref)
                .map(|b| b.origin.clone())
        });
        stored.resource_origin = stored.resource_origin.clone().or_else(|| {
            self.resource
                .as_ref()
                .ok()
                .and_then(Option::as_ref)
                .map(|b| b.origin.clone())
        });
        self.save(&entries)
    }

    pub(super) fn check_binding(&self, name: &str, stored: &Stored) -> Result<(), AuthError> {
        if let Ok(Some(binding)) = &self.binding {
            binding.check(name, stored.origin.as_deref())?;
        }
        if let Ok(Some(binding)) = &self.resource {
            binding.check(name, stored.resource_origin.as_deref())?;
        }
        Ok(())
    }

    pub(super) fn origin(&self) -> Option<String> {
        self.binding
            .as_ref()
            .ok()
            .and_then(Option::as_ref)
            .map(|b| b.origin.clone())
    }

    pub(super) fn resource_origin(&self) -> Option<String> {
        self.resource
            .as_ref()
            .ok()
            .and_then(Option::as_ref)
            .map(|b| b.origin.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Entry;

    #[test]
    fn invalid_addresses_cannot_read_write_or_adopt() {
        let dir = tempfile::tempdir().unwrap();
        let store = CredentialStore::new(dir.path().into());
        for address in ["not a URL", "https://", "file:///secret"] {
            let invalid = store.at_origin(address, "project.toml");
            assert!(
                invalid
                    .write(
                        "work",
                        Entry::Api {
                            key: "secret".into()
                        }
                    )
                    .is_err()
            );
            assert!(invalid.adopt("work").is_err());
            assert!(invalid.read("work").is_err());
            assert!(!store.path().exists());
        }
    }

    #[test]
    fn canonical_origins_pin_case_userinfo_ipv6_and_trailing_dot() {
        use crate::lifetime::origin;
        assert_eq!(
            origin("http://example.com"),
            Some("http://example.com:80".into())
        );
        assert_eq!(
            origin("http://example.com"),
            origin("http://example.com:80/path")
        );
        assert_eq!(
            origin("HTTPS://Example.COM/path"),
            Some("https://example.com:443".into())
        );
        assert_eq!(
            origin("https://user:SECRET@example.com"),
            origin("https://example.com")
        );
        assert_eq!(
            origin("http://[::1]:123/path"),
            Some("http://[::1]:123".into())
        );
        assert_ne!(
            origin("https://example.com."),
            origin("https://example.com")
        );
    }

    #[test]
    fn origin_checks_cover_ports_paths_and_every_read() {
        let dir = tempfile::tempdir().unwrap();
        let store = CredentialStore::new(dir.path().into()).owned_by("family");
        let first = store.at_origin("https://host/path", "user.toml");
        first
            .write(
                "work",
                Entry::Api {
                    key: "test-key".into(),
                },
            )
            .unwrap();
        assert_eq!(
            store.metadata("work").unwrap().unwrap().origin.as_deref(),
            Some("https://host:443")
        );
        assert!(
            store
                .at_origin("https://host:443/other", "project.toml")
                .read("work")
                .unwrap()
                .is_some()
        );
        for address in ["http://host", "https://host:444", "https://other"] {
            let changed = store.at_origin(address, "project.toml");
            let error = changed.read("work").unwrap_err().to_string();
            assert!(error.contains("project.toml") && !error.contains("test-key"));
        }
        store
            .at_origin("https://other", "user.toml")
            .write(
                "work",
                Entry::Api {
                    key: "replacement".into(),
                },
            )
            .unwrap();
        assert!(first.read("work").is_err());
    }
}
