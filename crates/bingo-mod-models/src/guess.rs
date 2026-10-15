#[derive(Clone, PartialEq, Eq)]
pub struct Guess {
    pub name: String,
    pub shape: String,
    /// Empty until someone says whose key this is.
    pub address: String,
    pub key: Option<String>,
}
/// A picker's state is logged and asserted on; the key it holds never is.
impl std::fmt::Debug for Guess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Guess")
            .field("name", &self.name)
            .field("shape", &self.shape)
            .field("address", &self.address)
            .field("key", &self.key.as_ref().map(|_| "[pasted]"))
            .finish()
    }
}
pub fn guess(value: &str, names: &[String]) -> Option<Guess> {
    let value = unwrapped(value);
    let value = value.as_str();
    let (name, shape, address, key) = if value.starts_with("sk-ant-") {
        (
            "anthropic".into(),
            "anthropic",
            "https://api.anthropic.com".into(),
            Some(value.into()),
        )
    } else if value.starts_with("sk-or-") {
        (
            "openrouter".into(),
            "openai",
            "https://openrouter.ai/api/v1".into(),
            Some(value.into()),
        )
    } else if value.starts_with("sk-") {
        // Many vendors mint `sk-` keys; the person says whose this is.
        (
            "provider".into(),
            "openai",
            String::new(),
            Some(value.into()),
        )
    } else {
        let address = address(value)?;
        let name = if value.contains(":11434") {
            "ollama"
        } else if value.contains(":1234") {
            "lmstudio"
        } else {
            "server"
        };
        (name.into(), "openai", address, None)
    };
    let mut unique: String = name;
    let base = unique.clone();
    let mut suffix = 2;
    while names.contains(&unique) {
        unique = format!("{base}-{suffix}");
        suffix += 1;
    }
    Some(Guess {
        name: unique,
        shape: shape.into(),
        address,
        key,
    })
}
/// What is kept of a pasted key: never the line breaks a wrapped paste
/// brings, and nothing when nothing is left.
pub fn key(pasted: &str) -> Option<String> {
    Some(unwrapped(pasted)).filter(|key| !key.is_empty())
}
fn unwrapped(pasted: &str) -> String {
    pasted
        .chars()
        .filter(|c| !matches!(c, '\r' | '\n'))
        .collect::<String>()
        .trim()
        .to_owned()
}
fn address(value: &str) -> Option<String> {
    if value.is_empty() || value.contains(char::is_whitespace) || value.contains(['@', '?', '#']) {
        return None;
    }
    let mut value = if value.starts_with("http://") || value.starts_with("https://") {
        value.to_owned()
    } else if value.starts_with("localhost:") || value.starts_with("127.0.0.1:") {
        format!("http://{value}")
    } else {
        return None;
    };
    if (value.contains(":11434") || value.contains(":1234"))
        && !value.trim_end_matches('/').ends_with("/v1")
    {
        value = format!("{}/v1", value.trim_end_matches('/'));
    }
    Some(value)
}
/// The OpenAI adapter appends its versioned paths to the configured root.
pub fn wire_address<'a>(shape: &str, address: &'a str) -> &'a str {
    let address = address.trim_end_matches('/');
    if shape == "openai" {
        address.strip_suffix("/v1").unwrap_or(address)
    } else {
        address
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prefixes_addresses_and_unique_names() {
        for (value, name, shape, key) in [
            ("sk-ant-dummy", "anthropic", "anthropic", true),
            ("sk-or-dummy", "openrouter", "openai", true),
            ("sk-dummy", "provider", "openai", true),
            ("localhost:11434", "ollama", "openai", false),
            ("http://localhost:1234", "lmstudio", "openai", false),
            ("https://example.invalid/v1", "server", "openai", false),
        ] {
            let guess = guess(value, &[name.into()]).unwrap();
            assert_eq!(guess.name, format!("{name}-2"));
            assert_eq!(guess.shape, shape);
            assert_eq!(guess.key.is_some(), key);
        }
        for value in [
            "",
            "a key",
            "https://user:secret@example.invalid",
            "https://example.invalid/?key=secret",
        ] {
            assert!(guess(value, &[]).is_none());
        }
    }
    #[test]
    fn a_bare_sk_key_names_no_vendor() {
        let guess = guess("sk-dummy", &[]).unwrap();
        assert_eq!(guess.address, "");
        assert_ne!(guess.name, "openai");
    }
    #[test]
    fn debug_never_shows_a_pasted_key() {
        let guess = guess("sk-ant-dummy-secret", &[]).unwrap();
        let printed = format!("{guess:?}");
        assert!(!printed.contains("sk-ant-dummy-secret"), "{printed}");
        assert!(printed.contains("[pasted]"), "{printed}");
    }
    #[test]
    fn a_wrapped_paste_keeps_one_clean_key() {
        assert_eq!(key("  sk-ant-du\r\nmmy\n "), Some("sk-ant-dummy".into()));
        assert_eq!(key(" \n "), None);
        let guess = guess("sk-ant-du\nmmy\n", &[]).unwrap();
        assert_eq!(guess.key.as_deref(), Some("sk-ant-dummy"));
    }
}
