//! The source of claimed settings, before field-by-field merging.
use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigOrigin {
    User,
    Repository,
    Explicit,
}

#[derive(Clone, Debug)]
pub struct ConfigLayer {
    pub source: String,
    pub origin: ConfigOrigin,
    pub values: Map<String, Value>,
}

#[derive(Clone, Debug, Default)]
pub struct ConfigSources {
    pub layers: Vec<ConfigLayer>,
    /// Counts across all families, without exposing another plugin's settings.
    pub user_instances: BTreeMap<String, usize>,
    /// Provider identities reserved by the composition, including disabled families.
    pub reserved_providers: BTreeSet<String>,
}

impl ConfigSources {
    pub fn source(&self, path: &[&str]) -> Option<&str> {
        self.layer(path).map(|layer| layer.source.as_str())
    }

    pub fn layer(&self, path: &[&str]) -> Option<&ConfigLayer> {
        self.layers
            .iter()
            .rev()
            .find(|layer| value(&layer.values, path).is_some())
    }

    pub fn user_value(&self, path: &[&str]) -> Option<&Value> {
        self.layers
            .iter()
            .rev()
            .filter(|layer| layer.origin == ConfigOrigin::User)
            .find_map(|layer| value(&layer.values, path))
    }
}

fn value<'a>(values: &'a Map<String, Value>, path: &[&str]) -> Option<&'a Value> {
    let (first, rest) = path.split_first()?;
    rest.iter()
        .try_fold(values.get(*first)?, |value, key| value.get(*key))
}
