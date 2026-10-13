//! The scenario file is a contract, including the fake provider's own script.
use bingo_provider_fake::Script;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub model: Script,
    pub steps: Vec<Step>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum Step {
    Submit { text: String },
    Reload {},
    Expect(Expectation),
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum Expectation {
    Slot { slot: String, contains: String },
    Tool { name: String, contains: String },
    Denied { name: String },
    Notice { contains: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_scenario_schema_is_generated() {
        let rendered = format!(
            "{}\n",
            serde_json::to_string_pretty(&schemars::schema_for!(Scenario)).unwrap()
        );
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../schema/plugin-scenario.json");
        if std::env::var_os("BINGO_UPDATE_SCHEMA").is_some() {
            std::fs::write(path, rendered).unwrap();
        } else {
            assert_eq!(
                std::fs::read_to_string(path).unwrap_or_default(),
                rendered,
                "scenario schema drift: run BINGO_UPDATE_SCHEMA=1 cargo test -p bingo committed_scenario_schema"
            );
        }
    }

    #[test]
    fn scenario_contract_names_every_step() {
        let value = serde_json::json!({"model":{"responses":[]},"steps":[
            {"submit":{"text":"hello"}}, {"reload":{}},
            {"expect":{"slot":{"slot":"calls","contains":"3"}}},
            {"expect":{"tool":{"name":"Read","contains":"hello"}}},
            {"expect":{"denied":{"name":"Bash"}}},
            {"expect":{"notice":{"contains":"reloaded"}}}
        ]});
        let scenario: Scenario = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(scenario).unwrap(), value);
        assert!(serde_json::from_value::<Step>(serde_json::json!({"reload":{"typo":1}})).is_err());
    }
}
