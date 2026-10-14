//! Inputs supplied by the embedding host.
use crate::{settings::Layer, turn::TurnBudget};
use bingo_sdk::Env;
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct HostConfig {
    /// Settings layers, lowest priority first; the command line is the last.
    pub layers: Vec<Layer>,
    /// Provider families shipped by the embedding binary, including disabled plugins.
    pub reserved_providers: std::collections::BTreeSet<String>,
    /// A system block appended after the kernel's own (hosts, tests).
    pub extra_system: Option<String>,
    pub budget: TurnBudget,
    pub env: Env,
    /// How deep a chain of sub-sessions may go; 1 = one level below a root.
    pub max_child_depth: u32,
    /// Live children one session may have; `None` places no bound.
    pub max_children: Option<usize>,
}

impl HostConfig {
    pub fn new(env: Env) -> Self {
        Self {
            layers: Vec::new(),
            reserved_providers: Default::default(),
            extra_system: None,
            budget: TurnBudget::default(),
            env,
            max_child_depth: 1,
            max_children: None,
        }
    }

    /// Add the highest-priority layer so far; a non-object is ignored.
    pub fn with_layer(mut self, source: &str, value: Value) -> Self {
        if let Value::Object(map) = value {
            self.layers.push(Layer::new(
                crate::settings::LayerOrigin::Explicit,
                source,
                map,
            ));
        }
        self
    }
}

impl super::Host {
    /// The settings layers used to resolve a live configuration change.
    pub(crate) fn settings_layers(&self) -> &[Layer] {
        &self.config.layers
    }
}

impl super::Host {
    pub(crate) fn current_trust(
        &self,
    ) -> Result<
        std::collections::BTreeMap<String, bingo_sdk::PluginTrust>,
        crate::settings::SettingsError,
    > {
        let path = crate::settings::user_path(self.env());
        let mut layers = self
            .settings_layers()
            .iter()
            .filter(|layer| !layer.is_user() || !std::path::Path::new(&layer.source).is_absolute())
            .cloned()
            .collect::<Vec<_>>();
        layers.push(Layer::new(
            crate::settings::LayerOrigin::User,
            path.display().to_string(),
            crate::settings::preview_document(&path)?,
        ));
        Ok(crate::settings::merge(&layers, &[])?.kernel.plugin_trust)
    }
}
