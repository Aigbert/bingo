//! Claimed values retain their provenance across the plugin boundary.
use super::Layer;
use bingo_sdk::PluginManifest;
use bingo_sdk::plugin::{ConfigLayer, ConfigOrigin, ConfigSources};

pub(crate) fn for_plugin(
    layers: &[Layer],
    manifest: &PluginManifest,
    reserved: &std::collections::BTreeSet<String>,
) -> ConfigSources {
    let mut sources = ConfigSources {
        reserved_providers: reserved.clone(),
        ..Default::default()
    };
    for layer in layers {
        if layer.is_user() {
            for family in layer.value.values() {
                if let Some(instances) = family.get("instances").and_then(|v| v.as_object()) {
                    for name in instances.keys() {
                        *sources.user_instances.entry(name.clone()).or_default() += 1;
                    }
                }
            }
        }
        let values = layer
            .value
            .iter()
            .filter(|(key, _)| {
                manifest.config.is_some_and(|claim| {
                    claim
                        .keys
                        .iter()
                        .any(|(path, _)| path.split('.').next() == Some(key.as_str()))
                })
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        sources.layers.push(ConfigLayer {
            source: layer.source.clone(),
            origin: if layer.is_user() {
                ConfigOrigin::User
            } else if layer.repository().is_some() {
                ConfigOrigin::Repository
            } else {
                ConfigOrigin::Explicit
            },
            values,
        });
    }
    sources
}

/// The just-written user document replaces the boot snapshot for a runtime edit.
pub(crate) fn for_instance(
    layers: &[Layer],
    user_path: &std::path::Path,
    key: &str,
    user: serde_json::Map<String, serde_json::Value>,
) -> ConfigSources {
    let user = Layer::new(
        super::LayerOrigin::User,
        user_path.display().to_string(),
        user,
    );
    let layers = std::iter::once(&user).chain(layers.iter().filter(|layer| !layer.is_user()));
    ConfigSources {
        layers: layers
            .map(|layer| ConfigLayer {
                source: layer.source.clone(),
                origin: if layer.is_user() {
                    ConfigOrigin::User
                } else if layer.repository().is_some() {
                    ConfigOrigin::Repository
                } else {
                    ConfigOrigin::Explicit
                },
                values: layer
                    .value
                    .iter()
                    .filter(|(name, _)| name.as_str() == key)
                    .map(|(name, value)| (name.clone(), value.clone()))
                    .collect(),
            })
            .collect(),
        ..Default::default()
    }
}
