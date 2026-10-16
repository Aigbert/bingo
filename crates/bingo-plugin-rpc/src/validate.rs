//! Validate a process plugin with the host's own spawner and handshake types.
use std::path::Path;

use bingo_sdk::{Env, ErrorCode, KernelError};
use serde_json::Value;

use crate::wire::{HostEnv, InitializeParams, InitializeResult, PROTOCOL};
use crate::{Connection, Manifest};

pub fn read(root: &Path) -> Result<Manifest, KernelError> {
    let path = root.join("plugin.json");
    let raw = std::fs::read_to_string(&path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => invalid(format!(
            "{}: not found; a plugin directory holds a plugin.json (see /guide-mods)",
            path.display()
        )),
        _ => invalid(format!("{}: {error}", path.display())),
    })?;
    let manifest: Manifest = serde_json::from_str(&raw)
        .map_err(|error| invalid(format!("{}: {error}", path.display())))?;
    let name = &manifest.name;
    if name.is_empty()
        || !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        || root.file_name().and_then(|name| name.to_str()) != Some(name.as_str())
    {
        return Err(invalid(
            "name must be a safe plugin directory name and match its directory",
        ));
    }
    if manifest
        .entry()
        .is_ok_and(|entry| entry.command.trim().is_empty())
    {
        return Err(invalid("entry.command must not be empty"));
    }
    if let Some(schema) = &manifest.config {
        check_schema(schema).map_err(|why| invalid(format!("config: {why}")))?;
    }
    Ok(manifest)
}

fn check_schema(schema: &Value) -> Result<(), String> {
    if schema.is_boolean() {
        return Ok(());
    }
    let Some(map) = schema.as_object() else {
        return Err("a JSON Schema is an object or boolean".into());
    };
    check_types(map)?;
    check_children(map)?;
    if let Some(required) = map.get("required")
        && !required
            .as_array()
            .is_some_and(|values| values.iter().all(Value::is_string))
    {
        return Err("required must be an array of strings".into());
    }
    for key in ["$ref", "$schema", "$id", "title", "description"] {
        if map.get(key).is_some_and(|value| !value.is_string()) {
            return Err(format!("{key} must be a string"));
        }
    }
    Ok(())
}

fn check_types(map: &serde_json::Map<String, Value>) -> Result<(), String> {
    if let Some(kind) = map.get("type") {
        let types = kind
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_else(|| std::slice::from_ref(kind));
        if types.is_empty()
            || types.iter().any(|kind| {
                !matches!(
                    kind.as_str(),
                    Some("null" | "boolean" | "object" | "array" | "number" | "integer" | "string")
                )
            })
        {
            return Err("type must name JSON Schema types".into());
        }
    }
    Ok(())
}

fn check_children(map: &serde_json::Map<String, Value>) -> Result<(), String> {
    for key in [
        "properties",
        "patternProperties",
        "$defs",
        "definitions",
        "dependentSchemas",
    ] {
        if let Some(value) = map.get(key) {
            let entries = value
                .as_object()
                .ok_or_else(|| format!("{key} must be an object"))?;
            for value in entries.values() {
                check_schema(value)?;
            }
        }
    }
    for key in [
        "items",
        "additionalProperties",
        "contains",
        "not",
        "if",
        "then",
        "else",
        "propertyNames",
        "unevaluatedProperties",
        "unevaluatedItems",
    ] {
        if let Some(value) = map.get(key) {
            check_schema(value)?;
        }
    }
    for key in ["oneOf", "anyOf", "allOf", "prefixItems"] {
        if let Some(value) = map.get(key) {
            let values = value
                .as_array()
                .filter(|values| !values.is_empty())
                .ok_or_else(|| format!("{key} must be a nonempty array"))?;
            for value in values {
                check_schema(value)?;
            }
        }
    }
    Ok(())
}

pub async fn validate(root: &Path, env: &Env) -> Result<Vec<String>, KernelError> {
    let root = root.canonicalize().map_err(invalid)?;
    let manifest = read(&root)?;
    let connection = Connection::spawn(
        &manifest.name,
        &manifest.entry().map_err(invalid)?.rooted(&root),
        &root,
        &env.data_dir,
        None,
    )
    .map_err(|why| invalid(format!("entry: {why}")))?;
    let result = tokio::time::timeout(
        crate::deadline::HANDSHAKE,
        initialize(&connection, &root, env, &manifest),
    )
    .await;
    connection.stop().await;
    let result = result.map_err(|_| invalid("initialize: handshake deadline exceeded"))??;
    findings(&result)
}

async fn initialize(
    connection: &Connection,
    root: &Path,
    env: &Env,
    manifest: &Manifest,
) -> Result<InitializeResult, KernelError> {
    let params = InitializeParams {
        protocol: PROTOCOL,
        plugin_root: root.into(),
        plugin_name: manifest.name.clone(),
        plugin_version: Some(manifest.version.clone()),
        capabilities: crate::wire::capabilities(),
        config: serde_json::json!({}),
        env: HostEnv::from(env),
    };
    let reply = connection
        .request("initialize", serde_json::to_value(params).map_err(invalid)?)
        .await
        .map_err(|why| invalid(format!("initialize: {}", why.message)))?;
    serde_json::from_value(reply).map_err(|why| invalid(format!("initialize: {why}")))
}

fn findings(result: &InitializeResult) -> Result<Vec<String>, KernelError> {
    if result.protocol != PROTOCOL {
        return Err(invalid(format!(
            "protocol: plugin speaks {}, host speaks {PROTOCOL}",
            result.protocol
        )));
    }
    let mut lines = vec![format!("{}: protocol {PROTOCOL}", result.name)];
    lines.extend(
        result
            .tools
            .iter()
            .map(|tool| format!("tool: {}", tool.name)),
    );
    lines.extend(
        result
            .commands
            .iter()
            .map(|command| format!("command: {}", command.name)),
    );
    lines.extend(result.links.iter().map(|link| {
        format!(
            "link: {} (after: {})",
            crate::intercept::point_name(link.point),
            link.after
        )
    }));
    lines.extend(subscription_findings(&result.subscriptions));
    Ok(lines)
}

fn subscription_findings(subscriptions: &[bingo_sdk::LinkMatcher]) -> Vec<String> {
    let mut lines: Vec<String> = subscriptions
        .iter()
        .flat_map(|subscription| {
            subscription
                .patterns
                .iter()
                .map(|pattern| format!("subscription: {}", String::from(pattern.clone())))
        })
        .collect();
    for entry in bingo_sdk::catalogue::ALL {
        if subscriptions.iter().any(|subscription| {
            subscription.patterns.is_empty()
                || subscription
                    .patterns
                    .iter()
                    .any(|pattern| pattern.matches(&entry.name))
        }) {
            let kind = match entry.kind {
                bingo_sdk::catalogue::Kind::Chain => "chain",
                bingo_sdk::catalogue::Kind::Observation => "observation",
            };
            lines.push(format!("{kind}: {}", entry.name));
        }
    }
    lines
}

fn invalid(why: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::InvalidInput, why.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_entry_and_bad_name_name_the_field() {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().join("example");
        std::fs::create_dir(&root).unwrap();
        for (value, field) in [
            (json!({"name":"example","version":"1"}), "entry"),
            (
                json!({"name":"wrong","version":"1","entry":{"command":"node"}}),
                "name",
            ),
            (
                json!({"name":"example","version":"1","entry":{"command":""}}),
                "entry",
            ),
            (
                json!({"name":"example","version":"1","entry":{"command":"node"},"config":{"type":"typo"}}),
                "config",
            ),
        ] {
            std::fs::write(root.join("plugin.json"), value.to_string()).unwrap();
            assert!(read(&root).unwrap_err().message.contains(field));
        }
    }

    #[test]
    fn a_missing_or_broken_manifest_names_its_path() {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().join("example");
        std::fs::create_dir(&root).unwrap();
        let path = root.join("plugin.json");
        let missing = read(&root).unwrap_err().message;
        assert!(missing.contains(&path.display().to_string()), "{missing}");
        assert!(missing.contains("not found"), "{missing}");
        std::fs::write(&path, "{").unwrap();
        let broken = read(&root).unwrap_err().message;
        assert!(broken.contains(&path.display().to_string()), "{broken}");
    }

    #[test]
    fn subscriptions_accept_every_catalogue_entry_and_dynamic_patterns() {
        for entry in bingo_sdk::catalogue::ALL {
            let result = serde_json::from_value(json!({
                "protocol": PROTOCOL, "name": "catalogue", "version": "1",
                "subscriptions": [{"patterns": [entry.name], "prefilter": null}]
            }))
            .unwrap();
            let lines = findings(&result).unwrap();
            assert!(
                lines.iter().any(|line| line.contains(entry.name.as_ref())),
                "missing catalogue subscription {}: {lines:?}",
                entry.name
            );
        }
        let result = serde_json::from_value(json!({
            "protocol": PROTOCOL, "name": "catalogue", "version": "1",
            "subscriptions": [{"patterns": ["*", "mcp.*"], "prefilter": null}]
        }))
        .unwrap();
        let lines = findings(&result).unwrap();
        for entry in bingo_sdk::catalogue::ALL {
            assert!(
                lines.iter().any(|line| line.contains(entry.name.as_ref())),
                "wildcard missed {}",
                entry.name
            );
        }
        assert!(lines.iter().any(|line| line.contains("mcp.*")));
    }

    #[test]
    fn protocol_one_names_both_numbers() {
        let result =
            serde_json::from_value(json!({"protocol":1,"name":"old","version":"1"})).unwrap();
        let error = findings(&result).unwrap_err();
        assert!(error.message.contains("protocol"));
        assert!(error.message.contains('1') && error.message.contains('2'));
    }
}
