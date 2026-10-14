//! The shared named-instance writer, using the comment-preserving settings edit.
use bingo_sdk::{ErrorCode, InstanceSpec, KernelError};
use serde_json::{Map, Value, json};
use std::path::Path;

pub fn validate_name(name: &str) -> Result<(), KernelError> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
    {
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

pub fn change(path: &Path, key: &str, name: &str, url: Option<&str>) -> Result<(), KernelError> {
    let mut document = read(path)?;
    let entries = instances(&mut document, key)?;
    let entry = entries
        .get_mut(name)
        .ok_or_else(|| invalid("only instances in user settings can be changed or removed"))?;
    if let Some(url) = url {
        entry
            .as_object_mut()
            .ok_or_else(|| invalid("instance settings must be an object"))?
            .insert("baseUrl".into(), json!(url));
    } else {
        entries.remove(name);
    }
    save(path, &document)
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
