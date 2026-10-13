//! Platform-owned configuration, construction and setup prompts.

use std::sync::Arc;

use bingo_sdk::Env;
use serde_json::{Map, Value};

use crate::adapter::ChannelAdapter;
use crate::secret::Requirement;

/// One public setting requested by `channels add`.
#[derive(Clone, Copy, Debug)]
pub struct Prompt {
    pub key: &'static str,
    pub label: &'static str,
    pub missing: &'static str,
    pub name: &'static str,
}

/// Setup text belongs to the platform whose credentials are being entered.
#[derive(Clone, Copy, Debug)]
pub struct Prompts {
    pub settings: &'static [Prompt],
    pub secret: &'static str,
}

/// The binary supplies the same platform values to every configuration reader.
pub trait Platform: Send + Sync {
    fn id(&self) -> &'static str;
    fn parse(&self, slice: &Value) -> Result<(), String>;
    fn build(&self, slice: &Value, env: &Env) -> Result<Arc<dyn ChannelAdapter>, String>;
    fn secret(&self) -> Option<Requirement>;
    fn flag(&self, value: Option<&str>) -> Value;
    fn prompts(&self) -> Prompts;
}

pub type Platforms = Vec<Arc<dyn Platform>>;

/// Validate configured platform slices and return their credential requirements.
pub fn configured(
    _slices: &Map<String, Value>,
    _platforms: &[Arc<dyn Platform>],
) -> Result<Vec<Requirement>, String> {
    Ok(Vec::new())
}

/// Resolve the platform named by a flag or settings slice.
pub fn find<'a>(_platforms: &'a [Arc<dyn Platform>], id: &str) -> Result<&'a dyn Platform, String> {
    Err(format!("no such channel adapter: {id}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_unknown_settings_slice_names_the_missing_platform() {
        let slices = Map::from_iter([("telegram".into(), json!({}))]);
        assert_eq!(
            configured(&slices, &[]),
            Err("no such channel adapter: telegram".into())
        );
    }

    #[test]
    fn a_platform_refusal_keeps_the_platforms_words() {
        let slices = Map::from_iter([("sample".into(), json!({"refuse": true}))]);
        let platforms: Platforms = vec![Arc::new(crate::testing::TestPlatform)];
        assert_eq!(
            configured(&slices, &platforms),
            Err("sample refuses this slice".into())
        );
    }

    #[test]
    fn an_unknown_flag_keeps_the_existing_refusal() {
        let error = find(&[], "telegram").err().expect("unknown platform");
        assert_eq!(error, "no such channel adapter: telegram");
    }
}
