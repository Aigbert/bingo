//! The source of claimed settings, before field-by-field merging.
use std::collections::BTreeMap;

use serde_json::{Map, Value};

#[derive(Clone, Debug)]
pub struct ConfigLayer {
    pub source: String,
    pub user: bool,
    pub values: Map<String, Value>,
}

#[derive(Clone, Debug, Default)]
pub struct ConfigSources {
    pub layers: Vec<ConfigLayer>,
    /// Counts across all families, without exposing another plugin's settings.
    pub user_instances: BTreeMap<String, usize>,
}

impl ConfigSources {
    pub fn source(&self, path: &[&str]) -> Option<&str> {
        self.layers
            .iter()
            .rev()
            .find_map(|layer| value(&layer.values, path).map(|_| layer.source.as_str()))
    }

    pub fn user_value(&self, path: &[&str]) -> Option<&Value> {
        self.layers
            .iter()
            .rev()
            .filter(|layer| layer.user)
            .find_map(|layer| value(&layer.values, path))
    }
}

fn value<'a>(values: &'a Map<String, Value>, path: &[&str]) -> Option<&'a Value> {
    let (first, rest) = path.split_first()?;
    rest.iter()
        .try_fold(values.get(*first)?, |value, key| value.get(*key))
}
