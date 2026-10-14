//! Facts supplied by the client; selection and argument navigation belong to the mod.
use crate::{Action, CatalogEntry, ConfigView, ContextUsage, SessionSummary};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PaletteOpen {
    #[default]
    Commands,
    Keys,
    Sessions,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaletteProps {
    pub query: String,
    pub open: PaletteOpen,
    pub commands: Vec<CatalogEntry>,
    pub key_hints: BTreeMap<String, String>,
    pub bindings: BTreeMap<String, String>,
    pub sessions: Vec<SessionSummary>,
    pub summary: SessionSummary,
    pub config: ConfigView,
    pub context: Option<ContextUsage>,
    pub signals: BTreeMap<String, BTreeMap<String, serde_json::Value>>,
    pub action: Action,
    pub catalogs: BTreeMap<String, Vec<CatalogEntry>>,
}
