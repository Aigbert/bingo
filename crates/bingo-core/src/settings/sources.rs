//! Claimed values retain their provenance across the plugin boundary.
use super::Layer;
use bingo_sdk::PluginManifest;
use bingo_sdk::plugin::{ConfigLayer, ConfigSources};

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
            user: layer.is_user(),
            values,
        });
    }
    sources
}
