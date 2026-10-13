//! The surface owns coalescing and access; each platform owns its settings.
use crate::{access::Access, adapter::ChannelAdapter, gate::Gate};
use bingo_channel_adapter::{
    Platforms,
    platform::{self, platform_slice},
};
use bingo_sdk::Env;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::{collections::BTreeMap, sync::Arc};

pub const SETTING: &str = "channels";

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct Settings {
    #[serde(default)]
    pub channels: Channels,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct Channels {
    #[serde(default)]
    pub coalesce: Coalesce,
    #[serde(flatten)]
    pub slices: Map<String, Value>,
}

#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Coalesce {
    /// New characters worth a redraw on their own.
    #[serde(default = "forty_eight")]
    pub min_chars: usize,
    /// The longest a person waits to see anything new.
    #[serde(default = "seven_hundred")]
    pub interval_ms: u64,
}

impl Default for Coalesce {
    fn default() -> Self {
        Self {
            min_chars: forty_eight(),
            interval_ms: seven_hundred(),
        }
    }
}

fn forty_eight() -> usize {
    48
}

fn seven_hundred() -> u64 {
    700
}

impl Channels {
    pub fn adapters(
        &self,
        env: &Env,
        platforms: &Platforms,
    ) -> Result<Vec<Arc<dyn ChannelAdapter>>, String> {
        platform::configured(&self.slices, platforms)?;
        self.slices
            .iter()
            .filter(|(_, slice)| !slice.is_null())
            .map(|(id, slice)| platform::find(platforms, id)?.build(&platform_slice(slice), env))
            .collect()
    }

    pub fn access(&self, platforms: &Platforms) -> Result<BTreeMap<String, Access>, String> {
        platform::configured(&self.slices, platforms)?;
        self.slices
            .iter()
            .filter(|(_, slice)| !slice.is_null())
            .map(|(id, slice)| {
                let access = match slice.get("access") {
                    Some(value) => {
                        serde_json::from_value(value.clone()).map_err(|e| e.to_string())?
                    }
                    None => Access::default(),
                };
                Ok((id.clone(), access))
            })
            .collect()
    }

    pub fn gate(&self, platforms: &Platforms) -> Result<Gate, String> {
        platform::configured(&self.slices, platforms)?;
        Ok(Gate {
            min_chars: self.coalesce.min_chars,
            interval: std::time::Duration::from_millis(self.coalesce.interval_ms),
        })
    }
}

pub fn wanted(settings: &Value, platforms: &Platforms) -> Result<bool, String> {
    Ok(!crate::secret::configured(settings, platforms)?.is_empty())
}

pub fn from_flags(flags: &[String], platforms: &Platforms) -> Result<Map<String, Value>, String> {
    if flags.is_empty() {
        return Ok(Map::new());
    }
    let mut channels = Map::new();
    for flag in flags {
        let (id, value) = match flag.split_once('=') {
            Some((id, value)) => (id, Some(value)),
            None => (flag.as_str(), None),
        };
        channels.insert(id.into(), platform::find(platforms, id)?.flag(value));
    }
    Ok(Map::from_iter([(SETTING.to_string(), channels.into())]))
}

pub fn schema() -> schemars::Schema {
    schemars::schema_for!(Settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn platforms() -> Platforms {
        vec![Arc::new(crate::loopback::Platform)]
    }

    fn parse(value: Value) -> Settings {
        serde_json::from_value(value).expect("the settings parse")
    }

    /// A home no test writes to: `adapters` reads a secret from the store, and
    /// none of these tests is about a secret.
    fn env() -> Env {
        Env::rooted("/nonexistent-bingo-settings-test")
    }

    #[test]
    fn nothing_configured_is_no_adapters_and_the_default_gate() {
        let settings = parse(json!({}));
        assert!(
            settings
                .channels
                .adapters(&env(), &platforms())
                .expect("adapters")
                .is_empty()
        );
        assert_eq!(
            settings.channels.gate(&platforms()).expect("gate"),
            Gate::default()
        );
        assert!(!wanted(&json!({}), &platforms()).expect("valid"));
        assert!(!wanted(&json!({ "channels": {} }), &platforms()).expect("valid"));
    }

    #[test]
    fn a_named_adapter_is_built_with_its_defaults() {
        let settings = parse(json!({ "channels": { "loopback": {} } }));
        let adapters = settings
            .channels
            .adapters(&env(), &platforms())
            .expect("adapters");
        assert_eq!(adapters.len(), 1);
        assert_eq!(adapters[0].id(), "loopback");
        assert!(adapters[0].edit().is_some(), "on unless turned off");
        assert!(wanted(&json!({ "channels": { "loopback": {} } }), &platforms()).expect("valid"));
    }

    #[test]
    fn a_mechanism_can_be_turned_off_from_the_settings() {
        let settings = parse(json!({
            "channels": { "loopback": { "edits": false, "buttons": false } }
        }));
        let adapters = settings
            .channels
            .adapters(&env(), &platforms())
            .expect("adapters");
        assert!(adapters[0].edit().is_none());
        assert!(adapters[0].buttons().is_none());
        assert!(adapters[0].typing().is_some());
        assert!(adapters[0].files().is_some());
    }

    #[test]
    fn a_policy_is_read_per_adapter_and_an_unnamed_channel_has_none() {
        let access = parse(json!({
            "channels": { "loopback": { "access": { "group": { "policy": "off" } } } }
        }))
        .channels
        .access(&platforms())
        .expect("access");
        assert_eq!(access["loopback"].group.policy, crate::access::Policy::Off);
        assert_eq!(
            access["loopback"].direct,
            crate::access::Rule::default(),
            "what a policy does not say keeps the default"
        );
        assert!(!access.contains_key("sample"));
    }

    #[test]
    fn an_adapter_that_says_nothing_about_access_runs_todays_rule() {
        let access = parse(json!({ "channels": { "loopback": {} } }))
            .channels
            .access(&platforms())
            .expect("access");
        assert_eq!(access["loopback"], Access::default());
    }

    #[test]
    fn the_gate_is_settable_because_a_test_should_not_have_to_wait() {
        let settings = parse(json!({
            "channels": { "coalesce": { "minChars": 1, "intervalMs": 5 } }
        }));
        assert_eq!(
            settings.channels.gate(&platforms()).expect("gate"),
            Gate {
                min_chars: 1,
                interval: std::time::Duration::from_millis(5),
            }
        );
    }

    #[test]
    fn a_flag_becomes_the_layer_the_plugin_would_have_read_from_a_file() {
        assert_eq!(
            Value::Object(
                from_flags(&["loopback=127.0.0.1:9".into()], &platforms()).expect("a layer")
            ),
            json!({ "channels": { "loopback": { "peer": "127.0.0.1:9" } } })
        );
        assert_eq!(
            Value::Object(from_flags(&["loopback".into()], &platforms()).expect("a layer")),
            json!({ "channels": { "loopback": { "peer": null } } })
        );
        assert!(from_flags(&[], &platforms()).expect("no flags").is_empty());
        assert_eq!(
            from_flags(&["telegram".into()], &platforms()),
            Err("no such channel adapter: telegram".into())
        );
    }
}
