//! Live facts and forwarding configuration, supplied by the MCP owner.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Status {
    Waiting { changed: bool },
    Connecting,
    Connected { tools: usize },
    SigningIn,
    NeedsAuth { why: String },
    Failed { why: String },
    Disabled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Auth {
    SignedOut,
    SignedIn,
    Expired { reason: String },
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Transport {
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: BTreeMap<String, String>,
        #[serde(default)]
        cwd: Option<PathBuf>,
    },
    Http {
        url: String,
        #[serde(default)]
        headers: BTreeMap<String, String>,
    },
}
impl Transport {
    pub fn name(&self) -> &str {
        match self {
            Self::Stdio { .. } => "stdio",
            Self::Http { .. } => "http",
        }
    }
    pub fn runs(&self) -> String {
        match self {
            Self::Stdio { command, args, .. } => std::iter::once(command.as_str())
                .chain(args.iter().map(String::as_str))
                .collect::<Vec<_>>()
                .join(" "),
            Self::Http { url, .. } => url.clone(),
        }
    }
}
impl std::fmt::Debug for Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transport")
            .field("type", &self.name())
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerRow {
    pub name: String,
    pub transport: Transport,
    pub origin: Option<String>,
    pub status: Status,
    pub auth: Option<Auth>,
    pub latency: Option<u64>,
    pub sign_in_from: Option<String>,
}
impl ServerRow {
    pub fn tools(&self) -> usize {
        match self.status {
            Status::Connected { tools } => tools,
            _ => 0,
        }
    }
    pub fn forwardable(&self) -> bool {
        !matches!(self.status, Status::Disabled | Status::Waiting { .. })
    }
}
