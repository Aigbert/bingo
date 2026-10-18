use crate::{
    CredentialStore, address,
    key::{Configured, Places},
};
use bingo_sdk::{PluginError, Registrar, plugin::ConfigSources};
use std::collections::BTreeSet;

pub struct InstanceConfig {
    pub reserved: &'static [&'static str],
    pub base_url: &'static str,
}

impl InstanceConfig {
    pub fn configured(
        &self,
        places: &Places,
        from_settings: Option<String>,
        sources: Option<&ConfigSources>,
        resolved: &str,
    ) -> Option<Configured> {
        let environment = places
            .variable
            .as_ref()
            .and_then(|variable| variable.base_url.as_deref());
        let permitted = |from_environment| {
            address::configured(
                sources,
                &places.setting,
                if sources.is_some() {
                    self.base_url
                } else {
                    resolved
                },
                environment,
                from_environment,
            )
        };
        if let Some(variable) = &places.variable
            && let Some(key) = blank(variable.api_key.clone())
        {
            return Some(Configured {
                key,
                place: variable.name.to_string(),
                address: Some(permitted(true)),
            });
        }
        blank(from_settings).map(|key| Configured {
            key,
            place: places.setting.clone(),
            address: Some(permitted(false)),
        })
    }

    pub fn allowed(
        &self,
        named: &mut BTreeSet<String>,
        name: &str,
        owner: &str,
        store: &CredentialStore,
        registrar: &mut Registrar,
    ) -> bool {
        let file = registrar
            .sources()
            .source(&[owner, "instances", name])
            .unwrap_or("settings")
            .to_string();
        if let Err(error) = self.claim(named, name) {
            let name = crate::lifetime::origin(name).unwrap_or_else(|| name.to_string());
            registrar.notice(
                "provider.instance",
                format!("Skipped instance `{name}` in {file}: {error}; rename or delete it there."),
            );
            return false;
        }
        if store.metadata(name).is_err() {
            registrar.notice(
                "provider.credentials",
                format!(
                    "{} is unreadable; stored keys are unavailable.",
                    store.path().display()
                ),
            );
            return true;
        }
        if let Some(entry) = store.metadata(name).ok().flatten()
            && entry.owner.as_deref() != Some(owner)
        {
            let held = entry.owner.as_deref().unwrap_or("an unknown family");
            registrar.notice("provider.instance", format!(
            "Skipped instance `{name}` in family `{owner}`: its credential belongs to `{held}`; rename or delete the `{owner}` declaration in {file}."
        ));
            return false;
        }
        true
    }

    pub fn claim(&self, named: &mut BTreeSet<String>, name: &str) -> Result<(), PluginError> {
        if !bingo_sdk::valid_provider_name(name) {
            return Err(PluginError::Config(
                "a provider name uses only ASCII letters, digits, `_` and `-`".into(),
            ));
        }
        if self.reserved.contains(&name) {
            return Err(PluginError::Config(format!(
                "provider instance `{name}` collides with the built-in provider of that name"
            )));
        }
        if !named.insert(name.to_string()) {
            return Err(PluginError::Config(format!(
                "provider instance `{name}` is named twice"
            )));
        }
        Ok(())
    }
}

pub fn blank(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_keys_use_explicit_variables_and_ignore_them_for_instances() {
        let config = InstanceConfig {
            reserved: &[],
            base_url: "https://default.example",
        };
        let setting = "family.apiKey";
        let address = "https://variable.example";
        for (variable, value, expected_key, expected_place, expected_address) in [
            (Some("API_KEY"), Some(" env "), "env", "API_KEY", address),
            (
                Some("API_KEY"),
                Some("  "),
                "settings-key",
                setting,
                address,
            ),
            (Some("API_KEY"), None, "settings-key", setting, address),
            (None, Some("env"), "settings-key", setting, config.base_url),
        ] {
            let places = Places {
                variable: variable.map(|name| {
                    std::sync::Arc::new(crate::key::Variable {
                        name,
                        api_key: value.map(str::to_string),
                        base_url: Some(address.into()),
                    })
                }),
                setting: "family.apiKey".into(),
                file: None,
            };
            let configured = config
                .configured(
                    &places,
                    Some(" settings-key ".into()),
                    None,
                    config.base_url,
                )
                .unwrap();
            assert_eq!(configured.key, expected_key);
            assert_eq!(configured.place, expected_place);
            assert_eq!(configured.address.as_deref(), Some(expected_address));
        }
    }
}
