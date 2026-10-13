//! The `plugin.json` a third party writes, and the one substitution it may use.
//!
//! Pure over strings and paths: nothing here touches the disk, so the manifest
//! a test builds and the manifest a directory holds are the same value.

use std::collections::BTreeMap;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The one placeholder a manifest may write, in `command`, in any of `args`
/// and in any `env` value: the directory the manifest itself was read from.
pub const PLUGIN_ROOT: &str = "${PLUGIN_ROOT}";

/// What one directory under `plugins/` declares about itself.
///
/// Unknown keys are kept rather than refused: a manifest written for a later
/// host must still start on this one.
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = exclusive_launch)]
pub struct Manifest {
    /// What the plugin calls itself. It must be the name of the directory
    /// holding it: the path is what one layer overrides another by, so two
    /// spellings of the name would be two answers to "which plugin is this".
    pub name: String,
    pub version: String,
    #[serde(flatten)]
    pub launch: Launch,
    /// A JSON Schema for this plugin's own settings — the slice a person
    /// writes under `plugins.<name>`, which reaches the process as
    /// `initialize.config`. It is what a person reads; nothing in this
    /// workspace validates a document against a schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<Value>,
    /// Maximum seconds for tool/call and command/run; absent allows ten minutes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_timeout: Option<std::num::NonZeroU64>,
}

fn exclusive_launch(schema: &mut schemars::Schema) {
    let map = schema.ensure_object();
    let properties = map.remove("properties").unwrap_or_default();
    let required = map.remove("required").unwrap_or_default();
    let Some(branches) = map.get_mut("oneOf").and_then(Value::as_array_mut) else {
        return;
    };
    let names: Vec<_> = branches
        .iter()
        .filter_map(|branch| branch["required"][0].as_str().map(str::to_owned))
        .collect();
    for branch in branches {
        let own = branch["required"][0]
            .as_str()
            .unwrap_or_default()
            .to_string();
        if let Some(fields) = branch["properties"].as_object_mut() {
            if let Some(common) = properties.as_object() {
                fields.extend(common.clone());
            }
            for name in &names {
                if name != &own {
                    fields.insert(name.clone(), Value::Bool(false));
                }
            }
        }
        if let (Some(fields), Some(common)) =
            (branch["required"].as_array_mut(), required.as_array())
        {
            fields.extend(common.clone());
        }
    }
}

/// Exactly one way for the host to run the plugin.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Launch {
    Entry(Entry),
    Module(String),
}

impl Manifest {
    pub fn entry(&self) -> Result<&Entry, String> {
        match &self.launch {
            Launch::Entry(entry) => Ok(entry),
            Launch::Module(_) => Err(format!("{}: module requires a module runtime", self.name)),
        }
    }
}

impl<'de> Deserialize<'de> for Manifest {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        let name = value
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("<unnamed>");
        parse(&value).map_err(|why| serde::de::Error::custom(format!("{name}: {why}")))
    }
}

fn parse(value: &Value) -> Result<Manifest, String> {
    let launch = match (value.get("entry"), value.get("module")) {
        (Some(entry), None) => {
            Launch::Entry(serde_json::from_value(entry.clone()).map_err(|e| e.to_string())?)
        }
        (None, Some(module)) => {
            Launch::Module(serde_json::from_value(module.clone()).map_err(|e| e.to_string())?)
        }
        _ => return Err("expected exactly one of entry or module".into()),
    };
    Ok(Manifest {
        name: field(value, "name")?,
        version: field(value, "version")?,
        launch,
        config: field(value, "config")?,
        call_timeout: field(value, "callTimeout")?,
    })
}

fn field<T: serde::de::DeserializeOwned>(value: &Value, key: &str) -> Result<T, String> {
    serde_json::from_value(value.get(key).cloned().unwrap_or(Value::Null))
        .map_err(|why| format!("{key}: {why}"))
}

/// The process to spawn.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// Added to the host's environment, never replacing it.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

impl Entry {
    /// The same entry with every `${PLUGIN_ROOT}` standing for `root`, which
    /// is how a manifest names the interpreter and the script beside itself
    /// without knowing where the directory was installed.
    pub fn rooted(&self, root: &Path) -> Entry {
        Entry {
            command: expand(&self.command, root),
            args: self.args.iter().map(|arg| expand(arg, root)).collect(),
            env: self
                .env
                .iter()
                .map(|(key, value)| (key.clone(), expand(value, root)))
                .collect(),
        }
    }
}

/// Every occurrence, not the first: a value may name the root twice.
fn expand(raw: &str, root: &Path) -> String {
    raw.replace(PLUGIN_ROOT, &root.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use serde_json::json;

    fn entry(command: &str, args: &[&str], env: &[(&str, &str)]) -> Entry {
        Entry {
            command: command.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            env: env
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect(),
        }
    }

    #[test]
    fn deadline_manifest_round_trips_the_override() {
        let manifest: Manifest = serde_json::from_value(json!({
            "name":"slow", "version":"1", "entry":{"command":"slow"}, "callTimeout":5
        }))
        .unwrap();
        assert_eq!(
            serde_json::to_value(manifest).unwrap()["callTimeout"],
            json!(5)
        );
    }

    #[test]
    fn a_manifest_is_the_four_keys_the_adr_names() {
        let manifest: Manifest = serde_json::from_value(json!({
            "name": "wordcount",
            "version": "0.1.0",
            "entry": { "command": "python3", "args": ["${PLUGIN_ROOT}/main.py"] }
        }))
        .expect("a manifest");
        assert_eq!(manifest.name, "wordcount");
        assert_eq!(manifest.entry().unwrap().command, "python3");
        assert!(manifest.entry().unwrap().env.is_empty());
        assert_eq!(manifest.config, None);
    }

    #[test]
    fn a_key_this_host_does_not_know_does_not_refuse_the_manifest() {
        let manifest: Manifest = serde_json::from_value(json!({
            "name": "later",
            "version": "9.0.0",
            "entry": { "command": "later" },
            "surfaces": ["one a later host will read"]
        }))
        .expect("a manifest from the future still parses");
        assert_eq!(manifest.name, "later");
    }

    #[test]
    fn the_root_reaches_the_command_the_arguments_and_the_environment() {
        let rooted = entry(
            "${PLUGIN_ROOT}/bin/run",
            &["--script", "${PLUGIN_ROOT}/main.py"],
            &[("PLUGIN_HOME", "${PLUGIN_ROOT}")],
        )
        .rooted(Path::new("/plugins/wordcount"));
        assert_eq!(rooted.command, "/plugins/wordcount/bin/run");
        assert_eq!(rooted.args[1], "/plugins/wordcount/main.py");
        assert_eq!(rooted.env["PLUGIN_HOME"], "/plugins/wordcount");
    }

    #[test]
    fn a_value_that_names_the_root_twice_has_both_resolved() {
        let rooted = entry("sh", &["-c", "${PLUGIN_ROOT}/a && ${PLUGIN_ROOT}/b"], &[])
            .rooted(&PathBuf::from("/p"));
        assert_eq!(rooted.args[1], "/p/a && /p/b");
    }

    #[test]
    fn an_entry_that_names_no_root_is_left_as_it_was() {
        let plain = entry("python3", &["main.py"], &[]);
        assert_eq!(plain.rooted(Path::new("/p")), plain);
    }
}
