//! Bind resolved endpoints while migrating only unambiguous user declarations.
use bingo_sdk::Registrar;
use serde_json::Value;

use crate::CredentialStore;

pub struct Endpoint<'a> {
    pub family: &'a str,
    pub name: &'a str,
    pub field: &'a str,
    pub default: &'a str,
    pub environment: Option<&'a str>,
    pub resource: Option<&'a str>,
}

impl Endpoint<'_> {
    fn path(&self) -> Vec<&str> {
        if self.family == self.name {
            vec![self.family]
        } else {
            vec![self.family, "instances", self.name]
        }
    }

    pub fn store(&self, registrar: &mut Registrar, store: &CredentialStore) -> CredentialStore {
        if self.family != self.name && !bingo_sdk::valid_provider_name(self.name) {
            return store.owned_by(self.family);
        }
        if store.metadata(self.name).is_err() {
            registrar.notice(
                "provider.credentials",
                format!(
                    "{} is unreadable; stored keys are unavailable.",
                    store.path().display()
                ),
            );
        }
        let config = registrar.config::<Value>().unwrap_or_default();
        let path = self.path();
        let resolved = path.iter().try_fold(&config, |v, key| v.get(*key));
        let mut scoped = store.owned_by(self.family);
        scoped = self.bind(registrar, &scoped, resolved, false);
        self.migrate(registrar, store);
        if let Err(crate::AuthError::Binding(message)) = scoped.read(self.name) {
            registrar.notice("provider.address", message);
        }
        scoped
    }

    fn bind(
        &self,
        registrar: &Registrar,
        store: &CredentialStore,
        value: Option<&Value>,
        user: bool,
    ) -> CredentialStore {
        let mut path = self.path();
        path.push(self.field);
        let file = if user {
            None
        } else {
            registrar.sources().source(&path)
        };
        let fallback = registrar.env().user_settings().display().to_string();
        let file = file.unwrap_or(&fallback);
        let environment = self
            .environment
            .and_then(|variable| std::env::var(variable).ok());
        let address = crate::address::resolve(
            value
                .and_then(|v| v.get(self.field))
                .and_then(Value::as_str),
            self.default,
            environment.as_deref(),
        );
        let mut scoped = store.at_origin(&address, file);
        if let Some(default) = self.resource {
            path.pop();
            path.push("baseUrl");
            let file = if user {
                None
            } else {
                registrar.sources().source(&path)
            }
            .unwrap_or(&fallback);
            let address = crate::address::resolve(
                value.and_then(|v| v.get("baseUrl")).and_then(Value::as_str),
                default,
                None,
            );
            scoped = scoped.at_resource(&address, file);
        }
        scoped
    }

    fn migrate(&self, registrar: &mut Registrar, store: &CredentialStore) {
        let path = self.path();
        let value = registrar.sources().user_value(&path);
        let allowed = if self.family == self.name {
            true
        } else {
            value.is_some()
                && registrar.sources().user_instances.get(self.name) == Some(&1)
                && !registrar.sources().reserved_providers.contains(self.name)
        };
        if !allowed {
            return;
        }
        let scoped = self.bind(registrar, &store.owned_by(self.family), value, true);
        if let Err(error) = scoped.adopt(self.name) {
            // An ownership collision is diagnosed by the instance claim, not migration.
            if store.metadata(self.name).is_ok_and(|entry| {
                entry.is_some_and(|e| e.owner.as_deref().is_none_or(|owner| owner == self.family))
            }) {
                registrar.notice("provider.credentials", error.to_string());
            }
        }
    }
}
