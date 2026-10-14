//! Construction shared by persisted and live named instances.
use crate::instances;
use bingo_auth_oauth::CredentialStore;
use bingo_sdk::{Env, InstanceSpec, PluginError, Provider, ProviderShape};
use std::sync::Arc;

pub(crate) struct Shape {
    http: reqwest::Client,
    env: Env,
}

impl Shape {
    pub(crate) fn new(http: reqwest::Client, env: &Env) -> Self {
        Self {
            http,
            env: env.clone(),
        }
    }
}

impl ProviderShape for Shape {
    fn id(&self) -> &str {
        "anthropic"
    }
    fn settings_key(&self) -> &str {
        self.id()
    }
    fn credential_exists(&self, name: &str) -> Result<bool, PluginError> {
        CredentialStore::new(self.env.data_dir.clone())
            .read(name)
            .map(|entry| entry.is_some())
            .map_err(|_| PluginError::Config("could not read credential store".into()))
    }
    fn forget_credential(&self, name: &str) -> Result<(), PluginError> {
        CredentialStore::new(self.env.data_dir.clone())
            .remove(name)
            .map_err(|_| PluginError::Config("could not remove stored credential".into()))
    }
    fn takes_key(&self) -> bool {
        true
    }
    fn build(&self, name: &str, spec: &InstanceSpec) -> Result<Arc<dyn Provider>, PluginError> {
        validate(spec)?;
        let store = Arc::new(CredentialStore::new(self.env.data_dir.clone()));
        let mut endpoint = crate::AnthropicEndpoint::default();
        if let Some(url) = &spec.base_url {
            endpoint.base_url = Some(url.clone());
        }
        if let Some(images) = spec.images {
            endpoint.images = images;
        }
        Ok(Arc::new(instances::keyed(
            self.http.clone(),
            name.to_string(),
            endpoint,
            &self.env.user_settings(),
            &store,
        )))
    }
}

pub(crate) fn validate(spec: &InstanceSpec) -> Result<(), PluginError> {
    if let Some(address) = &spec.base_url {
        let url = reqwest::Url::parse(address)
            .map_err(|_| PluginError::Config("invalid provider address".into()))?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(PluginError::Config(
                "use an http(s) address without credentials, query or fragment".into(),
            ));
        }
    }
    Ok(())
}
