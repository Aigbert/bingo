//! Facts supplied by the client; selection and argument navigation belong to the mod.
use crate::{Action, CatalogEntry, SessionId};
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
    pub session: SessionId,
    pub conversation: Vec<PaletteSession>,
    pub sessions: Vec<PaletteSession>,
    pub action: Action,
    /// Only the catalogues the query's command completes from, once the
    /// query reaches its arguments: every press carries them, and a model
    /// catalogue is the largest thing it would otherwise carry.
    pub catalogs: BTreeMap<String, Vec<CatalogEntry>>,
}

/// A compact row, projected by the client from live or stored facts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaletteSession {
    pub id: SessionId,
    pub label: String,
    pub detail: String,
    pub attention: Option<String>,
}
