//! Live replacements and removals; sessions keep their previously resolved Arc.
use bingo_sdk::{Provider, ProviderShape};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub(super) struct ProviderState {
    pub default_cleared: std::sync::atomic::AtomicBool,
    pub shapes: BTreeMap<String, Arc<dyn ProviderShape>>,
    pub instances: Instances,
    pub reach: super::reach::Reaches,
}

#[derive(Default)]
pub(super) struct Instances {
    entries: Mutex<BTreeMap<String, Option<Arc<dyn Provider>>>>,
    skipped: Mutex<BTreeMap<String, String>>,
    pub edit: tokio::sync::Mutex<()>,
    pub command: tokio::sync::Mutex<()>,
}

impl Instances {
    pub fn gather(
        &self,
        mut providers: Vec<Arc<dyn Provider>>,
        message: impl Fn(&dyn Provider, &dyn Provider) -> String,
    ) -> Vec<Arc<dyn Provider>> {
        let entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        providers.retain(|p| !p.instance() || !entries.contains_key(p.id()));
        for (name, entry) in entries.iter() {
            if let Some(winner) = providers.iter().find(|p| p.id() == name && !p.instance()) {
                if let Some(held) = entry {
                    self.skip(name, message(held.as_ref(), winner.as_ref()));
                }
            } else if let Some(provider) = entry {
                providers.push(provider.clone());
            }
        }
        providers
    }

    pub fn skip(&self, name: &str, message: String) {
        self.skipped
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(name.into(), message);
    }

    pub fn notices(&self) -> Vec<(String, String)> {
        self.skipped
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .values()
            .map(|message| ("provider.instance".into(), message.clone()))
            .collect()
    }

    pub fn current(&self, provider: &Arc<dyn Provider>) -> Option<bool> {
        self.entries
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(provider.id())
            .map(|entry| entry.as_ref().is_some_and(|p| Arc::ptr_eq(p, provider)))
    }

    pub fn replace(&self, name: &str, provider: Option<Arc<dyn Provider>>) {
        self.entries
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(name.into(), provider);
    }
}

/// Keep the address useful without disclosing URL credentials.
pub(super) fn address(raw: &str) -> Option<String> {
    let (scheme, rest) = raw.split_once("://")?;
    if !matches!(scheme, "http" | "https") {
        return None;
    }
    let rest = rest.split(['?', '#']).next()?;
    let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host = authority.rsplit('@').next()?;
    Some(format!(
        "{scheme}://{host}{}",
        if path.is_empty() {
            String::new()
        } else {
            format!("/{path}")
        }
    ))
}

pub(super) fn collision(
    instance: &dyn Provider,
    winner: &str,
    layers: &[crate::settings::Layer],
    fallback: &str,
) -> String {
    let name = instance.id();
    let family = instance.shape().unwrap_or_else(|| instance.family());
    let file = layers
        .iter()
        .rev()
        .find(|layer| {
            layer
                .value
                .get(family)
                .and_then(|v| v.get("instances"))
                .and_then(|v| v.get(name))
                .is_some()
        })
        .map(|layer| layer.source.as_str())
        .unwrap_or(fallback);
    format!(
        "Skipped instance `{name}` in family `{family}`: plugin `{winner}` provides that name; rename or delete the `{family}` declaration in {file}."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn address_drops_every_credential_component() {
        assert_eq!(
            address("https://user:secret@host:8443/v1?key=secret#secret"),
            Some("https://host:8443/v1".into())
        );
    }
}
