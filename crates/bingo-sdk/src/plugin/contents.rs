//! Contributions reported by the source that owns their running registration.
use crate::CommandSpec;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PluginContents {
    pub origin: Option<super::PluginOrigin>,
    pub runtime: Option<super::PluginRuntime>,
    pub commands: Vec<CommandSpec>,
    pub tools: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_contents_have_one_wire_shape() {
        let fixture = serde_json::json!({"origin":null,"runtime":null,"commands":[],"tools":[]});
        let contents: PluginContents = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(contents, PluginContents::default());
        assert_eq!(serde_json::to_value(contents).unwrap(), fixture);
    }
}
