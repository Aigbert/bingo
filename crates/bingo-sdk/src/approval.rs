//! Approval of repository code, resolved by the host that owns settings layers.
use crate::{KernelError, PluginState, ServiceKey};
use std::path::PathBuf;

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    pub root: PathBuf,
    pub what: String,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct Waiting {
    pub root: PathBuf,
    pub key: String,
    pub name: String,
    pub changed: bool,
}

pub trait ApprovalReader: Send + Sync {
    fn state(&self, request: &Approval) -> Result<PluginState, KernelError>;
}
pub const APPROVALS: ServiceKey<dyn ApprovalReader> = ServiceKey::new("bingo.approvals");

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn states_have_one_unambiguous_wire_representation() {
        for (state, value) in [
            (PluginState::On, json!({"kind":"on"})),
            (
                PluginState::Off { why: "off".into() },
                json!({"kind":"off","why":"off"}),
            ),
            (
                PluginState::Failed {
                    why: "broken".into(),
                },
                json!({"kind":"failed","why":"broken"}),
            ),
            (
                PluginState::Waiting { changed: false },
                json!({"kind":"waiting","changed":false}),
            ),
            (
                PluginState::Waiting { changed: true },
                json!({"kind":"waiting","changed":true}),
            ),
            (PluginState::Registering, json!({"kind":"pending"})),
            (
                PluginState::Pending {
                    now: false,
                    at: crate::plugin::NextStart::AfterTurn,
                },
                json!({"kind":"pending","now":false,"at":"afterTurn"}),
            ),
            (
                PluginState::Pending {
                    now: true,
                    at: crate::plugin::NextStart::NextStart,
                },
                json!({"kind":"pending","now":true,"at":"nextStart"}),
            ),
            (
                PluginState::Pending {
                    now: false,
                    at: crate::plugin::NextStart::NextStart,
                },
                json!({"kind":"pending","now":false,"at":"nextStart"}),
            ),
        ] {
            assert_eq!(serde_json::to_value(&state).unwrap(), value);
            assert_eq!(serde_json::from_value::<PluginState>(value).unwrap(), state);
        }
        for invalid in [
            json!({"enabled":true,"reason":null}),
            json!({"kind":"approved"}),
            json!({"kind":"waiting"}),
        ] {
            assert!(serde_json::from_value::<PluginState>(invalid).is_err());
        }
        let request = Approval {
            root: PathBuf::from("repo"),
            what: "plugin:hello".into(),
        };
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            json!({"root":"repo","what":"plugin:hello"})
        );
    }
}
