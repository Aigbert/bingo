//! Public, non-secret state of the model-choice site.
use crate::Effort;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModelPickerProps {
    pub filter: String,
    /// The chosen row's key; its position is found again on every drawing.
    pub selected: Option<String>,
    pub scope: ModelScope,
    pub effort: Option<Effort>,
    pub sheet: Option<String>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ModelScope {
    #[default]
    Session,
    Keep,
}
