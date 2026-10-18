//! Interaction chrome is declared by its owner, independent of a surface.
use super::*;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct KeyBarProps {
    pub pairs: Vec<KeyPair>,
}
impl KeyBarProps {
    /// The pairs that fit in `width` cells, kept whole from the right, and the
    /// cells they take with their ` · ` joins: a narrow row drops pairs from
    /// the left (ADR-0095). `cells` measures a string as the surface draws it.
    pub fn fitting(&self, width: usize, cells: impl Fn(&str) -> usize) -> (&[KeyPair], usize) {
        let mut used = 0;
        let mut start = self.pairs.len();
        for (index, pair) in self.pairs.iter().enumerate().rev() {
            let size = cells(&pair.keys)
                + usize::from(!pair.keys.is_empty())
                + cells(&pair.action)
                + usize::from(used > 0) * 3;
            if used + size > width {
                break;
            }
            used += size;
            start = index;
        }
        (&self.pairs[start..], used)
    }
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
    fn a_narrow_row_keeps_whole_pairs_from_the_right() {
        let props = KeyBarProps {
            pairs: vec![
                KeyPair::new("a", "bb"),
                KeyPair::new("", "ccc"),
                KeyPair::new("esc", "close"),
            ],
        };
        let fit = |width| {
            let (pairs, used) = props.fitting(width, str::len);
            (
                pairs.iter().map(|p| p.action.as_str()).collect::<Vec<_>>(),
                used,
            )
        };
        assert_eq!(fit(9), (vec!["close"], 9));
        assert_eq!(fit(14), (vec!["close"], 9));
        assert_eq!(fit(15), (vec!["ccc", "close"], 15));
        assert_eq!(fit(22), (vec!["bb", "ccc", "close"], 22));
        assert_eq!(fit(8), (Vec::<&str>::new(), 0));
    }
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
