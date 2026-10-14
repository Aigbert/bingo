//! Readdressing never carries an in-memory fallback into the replacement.
use super::{ApiKey, Configured};
use bingo_sdk::plugin::ConfigSources;

impl ApiKey {
    pub fn readdress(
        &self,
        owner: &str,
        address: &str,
        sources: &ConfigSources,
        default: &str,
        environment: Option<&str>,
    ) -> Self {
        let mut key = self.at_origin(owner, address, "user settings");
        key.configured = self
            .store
            .as_ref()
            .and_then(|_| self.reload(sources, default, environment));
        key
    }

    fn reload(
        &self,
        sources: &ConfigSources,
        default: &str,
        environment: Option<&str>,
    ) -> Option<Configured> {
        let variable = self.places.variable;
        let environment = variable.and(environment);
        let from_env = variable
            .and_then(|name| std::env::var(name).ok())
            .filter(|key| !key.trim().is_empty());
        let path: Vec<_> = self.places.setting.split('.').collect();
        let setting = sources
            .layers
            .iter()
            .rev()
            .find_map(|layer| crate::address::value(&layer.values, &path))
            .and_then(serde_json::Value::as_str);
        let key = from_env.as_deref().or(setting)?.trim();
        if key.is_empty() {
            return None;
        }
        Some(Configured {
            key: key.into(),
            place: from_env
                .as_ref()
                .and(variable)
                .unwrap_or(&self.places.setting)
                .into(),
            address: Some(crate::address::configured(
                Some(sources),
                &self.places.setting,
                default,
                environment,
                from_env.is_some(),
            )),
        })
    }
}
