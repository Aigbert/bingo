//! Facts supplied by the client; selection and argument navigation belong to the mod.
use crate::{Action, CatalogEntry, Driver, SessionId};
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
    /// What the session on screen is, so the list offers what works there.
    #[serde(default)]
    pub here: PaletteHere,
}

/// The session on screen as the list needs it: what answers it, and whether
/// it still takes input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaletteHere {
    pub driver: Driver,
    #[serde(default)]
    pub closed: bool,
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
