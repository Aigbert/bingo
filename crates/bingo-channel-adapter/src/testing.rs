//! A platform whose configuration refusal is independent of the surface.

use crate::adapter::ChannelAdapter;
use crate::platform::{Platform, Prompts};
use crate::secret::Requirement;
use bingo_sdk::Env;
use serde_json::{Value, json};
use std::sync::Arc;

pub struct TestPlatform;

impl Platform for TestPlatform {
    fn id(&self) -> &'static str {
        "sample"
    }
    fn parse(&self, slice: &Value) -> Result<(), String> {
        if slice.get("refuse") == Some(&Value::Bool(true)) {
            Err("sample refuses this slice".into())
        } else {
            Ok(())
        }
    }
    fn build(&self, _slice: &Value, _env: &Env) -> Result<Arc<dyn ChannelAdapter>, String> {
        Err("sample has no transport".into())
    }
    fn secret(&self) -> Option<Requirement> {
        Some(Requirement {
            id: "sample",
            variable: Some("BINGO_TEST_SAMPLE_SECRET"),
        })
    }
    fn flag(&self, value: Option<&str>) -> Value {
        json!({"value": value})
    }
    fn prompts(&self) -> Prompts {
        Prompts {
            settings: &[],
            secret: "",
        }
    }
}
