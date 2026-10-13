//! Conservative host matching: an unsupported predicate always crosses.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub struct Pattern(String);

impl Pattern {
    pub fn parse(text: &str) -> Result<Self, String> {
        let positive = text.trim_start_matches('!');
        let name = positive.strip_suffix(".*").unwrap_or(positive);
        if positive == "*"
            || (!name.is_empty()
                && name.split('.').all(|part| {
                    !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                }))
        {
            Ok(Self(text.into()))
        } else {
            Err(format!("invalid event pattern: {text}"))
        }
    }
    pub fn matches(&self, name: &str) -> bool {
        matches(&self.0, name)
    }
}
fn matches(pattern: &str, name: &str) -> bool {
    if let Some(rest) = pattern.strip_prefix('!') {
        return !matches(rest, name);
    }
    pattern == "*"
        || pattern
            .strip_suffix('*')
            .map_or(pattern == name, |prefix| name.starts_with(prefix))
}
impl TryFrom<String> for Pattern {
    type Error = String;
    fn try_from(text: String) -> Result<Self, String> {
        Self::parse(&text)
    }
}
impl From<Pattern> for String {
    fn from(pattern: Pattern) -> String {
        pattern.0
    }
}

pub fn prefilter(scalars: &Value, event: &Value) -> bool {
    match scalars {
        Value::Object(fields) => fields
            .iter()
            .all(|(key, value)| event.get(key).is_none_or(|found| prefilter(value, found))),
        Value::Array(_) => true,
        scalar => scalar == event,
    }
}

pub fn pinned_changed<'a>(before: &Value, after: &Value, pinned: &'a [&str]) -> Option<&'a str> {
    pinned
        .iter()
        .copied()
        .find(|path| before.pointer(path) != after.pointer(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_patterns_and_scalar_matchers() {
        let cases: Value =
            serde_json::from_str(include_str!("../../../../sdk/typescript/test/cases.json"))
                .unwrap();
        for case in cases["patterns"].as_array().unwrap() {
            assert_eq!(
                Pattern::parse(case[0].as_str().unwrap())
                    .unwrap()
                    .matches(case[1].as_str().unwrap()),
                case[2]
            );
        }
        for case in cases["matchers"].as_array().unwrap() {
            if case[2] == true {
                assert!(prefilter(&case[0], &case[1]), "{case}");
            }
        }
    }
    #[test]
    fn pinned_missing_and_changed_fields_are_reported() {
        let before = serde_json::json!({"tool":"Read","tool_use_id":"1"});
        assert_eq!(pinned_changed(&before, &before, &["/tool"]), None);
        assert_eq!(
            pinned_changed(&before, &Value::Null, &["/tool"]),
            Some("/tool")
        );
    }
}
