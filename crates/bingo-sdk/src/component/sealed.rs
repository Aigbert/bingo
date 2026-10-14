use crate::{AnswerSpec, InteractionId, LoginFlow};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PermissionProps {
    pub interaction_id: InteractionId,
    pub tool: String,
    pub input: Value,
    pub answers: Vec<AnswerSpec>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SignInProps {
    pub interaction_id: InteractionId,
    pub provider: String,
    pub flow: LoginFlow,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::Props;

    #[test]
    fn sealed_props_round_trip_the_fact_fixtures() {
        let fixtures: Vec<Value> = serde_json::from_str(include_str!("sealed.json")).unwrap();
        assert_eq!(fixtures.len(), 4);
        for fixture in fixtures {
            let props: Props = serde_json::from_value(fixture.clone()).unwrap();
            assert!(props.component().sealed());
            assert_eq!(serde_json::to_value(&props).unwrap(), fixture);
            assert_eq!(
                Props::parse(props.component(), props.value()).unwrap(),
                props
            );
        }
    }
}
