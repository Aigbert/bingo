//! Bounded, framed directory fingerprints. Links contribute text, never target bytes.
use serde_json::Value;
use std::{io::Read, path::Path};

const BYTES: usize = 8 * 1024 * 1024;
const FILES: usize = 2_000;

pub fn digest_entry(value: &Value) -> String {
    hash(canonical(value).to_string().as_bytes())
}
fn canonical(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let sorted: std::collections::BTreeMap<_, _> = object.iter().collect();
            Value::Object(
                sorted
                    .into_iter()
                    .map(|(k, v)| (k.clone(), canonical(v)))
                    .collect(),
            )
        }
        Value::Array(array) => Value::Array(array.iter().map(canonical).collect()),
        other => other.clone(),
    }
}
fn hash(bytes: &[u8]) -> String {
    format!("sha256:{}", bingo_digest::hex(&bingo_digest::digest(bytes)))
}
fn field(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    out.extend_from_slice(bytes);
}
fn path_text(path: &Path) -> std::io::Result<String> {
    let text = path
        .to_str()
        .ok_or_else(|| invalid("non-UTF-8 path cannot be approved"))?;
    portable(text, cfg!(windows))
}
fn portable(text: &str, windows: bool) -> std::io::Result<String> {
    // A literal backslash on Unix must not alias a Windows path separator.
    if !windows && text.contains('\\') {
        return Err(invalid("literal backslash in a path cannot be approved"));
    }
    Ok(text.replace('\\', "/"))
}

fn invalid(why: &str) -> std::io::Error {
    std::io::Error::other(why)
}

pub fn digest_dir(root: &Path) -> std::io::Result<String> {
    if !std::fs::symlink_metadata(root)?.is_dir() {
        return Err(invalid("approval requires a real plugin directory"));
    }
    let mut rows = Vec::new();
    let mut bytes = 0;
    walk(root, Path::new(""), &mut rows, &mut bytes)?;
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = b"bingo-directory-v1\0".to_vec();
    for (path, kind, content) in rows {
        field(&mut out, path.as_bytes());
        out.push(kind);
        field(&mut out, &content);
    }
    Ok(hash(&out))
}
type Row = (String, u8, Vec<u8>);
fn walk(
    root: &Path,
    relative: &Path,
    rows: &mut Vec<Row>,
    bytes: &mut usize,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(root.join(relative))? {
        let entry = entry?;
        let relative = relative.join(entry.file_name());
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)?;
        if rows.len() >= FILES {
            return Err(invalid("plugin exceeds 2,000 entries"));
        }
        let name = path_text(&relative)?;
        let (kind, content) = if metadata.file_type().is_symlink() {
            (b'l', path_text(&std::fs::read_link(&path)?)?.into_bytes())
        } else if metadata.is_dir() {
            (b'd', Vec::new())
        } else if metadata.is_file() {
            let mut content = Vec::new();
            std::fs::File::open(&path)?
                .take((BYTES - *bytes + 1) as u64)
                .read_to_end(&mut content)?;
            (b'f', content)
        } else {
            return Err(invalid("special files cannot be approved"));
        };
        *bytes += content.len() + name.len();
        if *bytes > BYTES {
            return Err(invalid("plugin exceeds 8 MiB"));
        }
        rows.push((name, kind, content));
        if kind == b'd' {
            walk(root, &relative, rows, bytes)?;
        }
    }
    Ok(())
}

/// A link's bytes are fingerprinted, but unbounded external code is never approved.
pub fn contained(root: &Path) -> std::io::Result<()> {
    let canonical = root.canonicalize()?;
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let metadata = std::fs::symlink_metadata(entry.path())?;
            if metadata.file_type().is_symlink() {
                let target = entry.path().canonicalize()?;
                if !target.starts_with(&canonical) {
                    return Err(invalid("symlink escapes the plugin directory"));
                }
            } else if metadata.is_dir() {
                pending.push(entry.path());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn json_order_is_irrelevant_but_bytes_and_paths_are_not() {
        assert_eq!(
            digest_entry(&serde_json::json!({"b":1,"a":{"d":2,"c":3}})),
            digest_entry(&serde_json::json!({"a":{"c":3,"d":2},"b":1}))
        );
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a"), "abc").unwrap();
        let before = digest_dir(dir.path()).unwrap();
        std::fs::write(dir.path().join("a"), "abd").unwrap();
        assert_ne!(before, digest_dir(dir.path()).unwrap());
        std::fs::write(dir.path().join("a"), "abc").unwrap();
        std::fs::rename(dir.path().join("a"), dir.path().join("b")).unwrap();
        assert_ne!(before, digest_dir(dir.path()).unwrap());
        assert_eq!(
            portable("nested\\file", true).unwrap(),
            portable("nested/file", false).unwrap()
        );
        assert!(portable("nested\\file", false).is_err());
    }
    #[test]
    fn caps_fail_closed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("large"), vec![0; BYTES + 1]).unwrap();
        assert!(
            digest_dir(dir.path())
                .unwrap_err()
                .to_string()
                .contains("8 MiB")
        );
        std::fs::remove_file(dir.path().join("large")).unwrap();
        for i in 0..=FILES {
            std::fs::write(dir.path().join(i.to_string()), []).unwrap();
        }
        assert!(
            digest_dir(dir.path())
                .unwrap_err()
                .to_string()
                .contains("2,000")
        );
    }
    #[cfg(unix)]
    #[test]
    fn links_hash_their_text_without_following_loops_or_outside_targets() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        symlink(outside.path(), dir.path().join("out")).unwrap();
        symlink("loop", dir.path().join("loop")).unwrap();
        let before = digest_dir(dir.path()).unwrap();
        std::fs::write(outside.path(), "changed").unwrap();
        assert_eq!(before, digest_dir(dir.path()).unwrap());
        assert!(contained(dir.path()).is_err());
        std::fs::remove_file(dir.path().join("out")).unwrap();
        assert!(contained(dir.path()).is_err(), "a loop alone is refused");
        std::fs::remove_file(dir.path().join("loop")).unwrap();
        symlink(outside.path(), dir.path().join("out")).unwrap();
        assert!(contained(dir.path()).is_err(), "an escape alone is refused");
    }
}
