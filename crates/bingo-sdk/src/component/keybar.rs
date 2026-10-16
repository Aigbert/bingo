//! Interaction chrome is declared by its owner, independent of a surface.
use super::*;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct KeyBarProps {
    pub pairs: Vec<KeyPair>,
    pub toast: Option<KeyToast>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ComposerProps {
    pub tag: String,
    pub placeholder: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct KeyPair {
    pub keys: String,
    pub action: String,
    #[serde(default)]
    pub emphasis: bool,
}
impl KeyPair {
    pub fn new(keys: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            keys: keys.into(),
            action: action.into(),
            emphasis: false,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SiteHints {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pairs: Vec<KeyPair>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composer: Option<ComposerProps>,
}
impl SiteHints {
    pub fn new(pairs: impl IntoIterator<Item = KeyPair>) -> Self {
        Self {
            pairs: pairs.into_iter().collect(),
            composer: None,
        }
    }
    pub fn composer(mut self, tag: impl Into<String>, placeholder: impl Into<String>) -> Self {
        self.composer = Some(ComposerProps {
            tag: tag.into(),
            placeholder: placeholder.into(),
        });
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct KeyToast {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    pub text: String,
    pub level: crate::Level,
}

/// A question owner's editing projection; sealed labels remain the host's.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct QuestionFocus {
    pub field: usize,
    pub checked: usize,
    pub writing: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pane_hints_are_optional_and_round_trip() {
        let wire = serde_json::json!({"pairs":[{"keys":"⏎","action":"use here","emphasis":true}],"composer":{"tag":"/model ","placeholder":"filter models"}});
        let hints: SiteHints = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(hints).unwrap(), wire);
        assert_eq!(
            serde_json::from_value::<SiteHints>(serde_json::json!({})).unwrap(),
            SiteHints::default()
        );
    }
}
