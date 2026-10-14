//! The panel's opening and client-minted command-line destination.
use crate::{Action, CatalogEntry, SessionId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PluginsOpen {
    #[default]
    Plugins,
    Mcp,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PluginsProps {
    pub open: PluginsOpen,
    pub session: SessionId,
    pub plugins: Vec<CatalogEntry>,
    pub action: Action,
}
