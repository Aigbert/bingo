//! The shared named-instance writer, using the comment-preserving settings edit.
use bingo_sdk::{ErrorCode, InstanceSpec, KernelError};
use serde_json::{Map, Value, json};
use std::path::Path;

pub fn validate_name(name: &str) -> Result<(), KernelError> {
    if !bingo_sdk::valid_provider_name(name) {
        return Err(invalid(
            "a provider name uses only ASCII letters, digits, `_` and `-`",
        ));
    }
    Ok(())
}

pub fn insert(
    document: &mut Map<String, Value>,
    key: &str,
    name: &str,
    spec: &InstanceSpec,
) -> Result<(), KernelError> {
    validate_name(name)?;
    if document
        .values()
        .any(|v| v.get("instances").and_then(|v| v.get(name)).is_some())
    {
        return Err(invalid("that provider name already exists in the settings"));
    }
    let value = serde_json::to_value(spec).map_err(|_| invalid("invalid instance spec"))?;
    instances(document, key)?.insert(name.into(), value);
    Ok(())
}

pub fn add(path: &Path, key: &str, name: &str, spec: &InstanceSpec) -> Result<(), KernelError> {
    let mut document = read(path)?;
    insert(&mut document, key, name, spec)?;
    save(path, &document)
}

pub fn change(
    path: &Path,
    key: &str,
    name: &str,
    url: Option<&str>,
) -> Result<Vec<String>, KernelError> {
    edit(path, key, name, url, false).map(|(_, cleared)| cleared)
}

pub fn ensure_user_key(layers: &[super::Layer], key: &str, name: &str) -> Result<(), KernelError> {
    for layer in layers.iter().filter(|layer| !layer.is_user()) {
        let held = layer
            .value
            .get(key)
            .and_then(|v| v.get("instances"))
            .and_then(|v| v.get(name))
            .and_then(|v| v.get("apiKey"));
        if held.is_some_and(|v| !v.is_null()) {
            return Err(invalid(&format!(
                "address unchanged; remove the instance apiKey from {} first",
                layer.source
            )));
        }
    }
    Ok(())
}

pub fn ensure_user(path: &Path, key: &str, name: &str) -> Result<(), KernelError> {
    if read(path)?
        .get(key)
        .and_then(|v| v.get("instances"))
        .and_then(|v| v.get(name))
        .is_none()
    {
        return Err(invalid(
            "only instances in user settings can be changed or removed",
        ));
    }
    Ok(())
}

pub fn change_address(
    path: &Path,
    key: &str,
    name: &str,
    url: &str,
    clear_key: bool,
) -> Result<Map<String, Value>, KernelError> {
    edit(path, key, name, Some(url), clear_key).map(|(document, _)| document)
}

fn edit(
    path: &Path,
    key: &str,
    name: &str,
    url: Option<&str>,
    clear_key: bool,
) -> Result<(Map<String, Value>, Vec<String>), KernelError> {
    let mut document = read(path)?;
    let entries = instances(&mut document, key)?;
    let entry = entries
        .get_mut(name)
        .ok_or_else(|| invalid("only instances in user settings can be changed or removed"))?;
    if clear_key {
        entry
            .as_object_mut()
            .ok_or_else(|| invalid("instance settings must be an object"))?
            .remove("apiKey");
    }
    if let Some(url) = url {
        entry
            .as_object_mut()
            .ok_or_else(|| invalid("instance settings must be an object"))?
            .insert("baseUrl".into(), json!(url));
    } else {
        entries.remove(name);
    }
    let cleared = if url.is_none() {
        clear_choices(&mut document, name)
    } else {
        Vec::new()
    };
    save(path, &document)?;
    Ok((document, cleared))
}

fn clear_choices(document: &mut Map<String, Value>, name: &str) -> Vec<String> {
    let mut cleared = Vec::new();
    if document.get("provider").and_then(Value::as_str) == Some(name) {
        document.remove("provider");
        if document.remove("model").is_some() {
            cleared.push("default model".into());
        }
        cleared.push("default provider".into());
    }
    if let Some(pins) = document.get_mut("pinned").and_then(Value::as_array_mut) {
        let before = pins.len();
        let prefix = format!("{name}/");
        pins.retain(|pin| !pin.as_str().is_some_and(|p| p.starts_with(&prefix)));
        if pins.len() != before {
            cleared.push(format!("{} model pins", before - pins.len()));
        }
    }
    cleared
}

fn instances<'a>(
    document: &'a mut Map<String, Value>,
    key: &str,
) -> Result<&'a mut Map<String, Value>, KernelError> {
    document
        .entry(key)
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| invalid("provider settings must be an object"))?
        .entry("instances")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| invalid("provider instances must be an object"))
}

fn read(path: &Path) -> Result<Map<String, Value>, KernelError> {
    super::read_document(path)
        .map_err(|_| invalid("could not read user settings; the file was left unchanged"))
}

fn save(path: &Path, document: &Map<String, Value>) -> Result<(), KernelError> {
    super::write(path, document).map_err(|_| invalid("could not write user settings"))
}

fn invalid(message: &str) -> KernelError {
    KernelError::new(ErrorCode::InvalidInput, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn removal_clears_the_default_and_only_this_instances_pins() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        std::fs::write(
            &path,
            r#"model = "m"
provider = "work"
# keep
pinned = ["work/m", "other/m", "worker/m"]
[local.instances.work]
baseUrl = "http://up"
"#,
        )
        .unwrap();
        change(&path, "local", "work", None).unwrap();
        let doc = read(&path).unwrap();
        assert!(doc.get("provider").is_none());
        assert!(doc.get("model").is_none());
        assert_eq!(doc["pinned"], json!(["other/m", "worker/m"]));
        assert!(std::fs::read_to_string(path).unwrap().contains("# keep"));
    }

    #[test]
    fn names_cannot_address_other_credential_namespaces() {
        for name in ["channels.telegram", "service:key", "é", "a/b", "a b", ""] {
            assert!(validate_name(name).is_err(), "{name}");
        }
        assert!(validate_name("Work_2-local").is_ok());
    }

    #[test]
    fn edits_keep_comments_and_neighbours_and_refuse_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        std::fs::write(&path, "# retained\nmodel = \"m\"\n").unwrap();
        let spec = InstanceSpec {
            base_url: Some("http://localhost:1".into()),
            images: None,
        };
        add(&path, "openai", "local", &spec).unwrap();
        assert!(add(&path, "anthropic", "local", &spec).is_err());
        change(&path, "openai", "local", Some("http://localhost:2")).unwrap();
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .starts_with("# retained\nmodel = \"m\"\n")
        );
        assert_eq!(
            read(&path).unwrap()["openai"]["instances"]["local"]["baseUrl"],
            "http://localhost:2"
        );
        change(&path, "openai", "local", None).unwrap();
        assert!(
            Value::Object(read(&path).unwrap())["openai"]["instances"]
                .get("local")
                .is_none()
        );
    }
}
