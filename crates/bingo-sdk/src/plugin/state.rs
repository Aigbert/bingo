//! Admission and switch progress share the legacy pending wire tag.
use super::{NextStart, PluginState};

#[derive(serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum WireState {
    On,
    Off {
        why: String,
    },
    Failed {
        why: String,
    },
    Waiting {
        changed: bool,
    },
    #[serde(alias = "registering")]
    Pending {
        now: Option<bool>,
        at: Option<NextStart>,
    },
}
impl<'de> serde::Deserialize<'de> for PluginState {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match WireState::deserialize(deserializer)? {
            WireState::On => Self::On,
            WireState::Off { why } => Self::Off { why },
            WireState::Failed { why } => Self::Failed { why },
            WireState::Waiting { changed } => Self::Waiting { changed },
            WireState::Pending {
                now: None,
                at: None,
            } => Self::Registering,
            WireState::Pending {
                now: Some(now),
                at: Some(at),
            } => Self::Pending { now, at },
            WireState::Pending { .. } => {
                return Err(serde::de::Error::custom(
                    "pending switch needs both now and at",
                ));
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::plugin::{NextStart, PluginState};
    use serde_json::json;

    #[derive(Debug, PartialEq, serde::Deserialize)]
    #[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
    enum LegacyState {
        Pending,
    }

    #[test]
    fn pending_wire_remains_readable_by_legacy_clients() {
        for state in [
            PluginState::Registering,
            PluginState::Pending {
                now: false,
                at: NextStart::NextStart,
            },
        ] {
            let wire = serde_json::to_value(&state).unwrap();
            assert_eq!(wire["kind"], "pending");
            assert_eq!(
                serde_json::from_value::<LegacyState>(wire.clone()).unwrap(),
                LegacyState::Pending
            );
            assert_eq!(serde_json::from_value::<PluginState>(wire).unwrap(), state);
        }
        assert_eq!(
            serde_json::from_value::<PluginState>(json!({"kind":"pending"})).unwrap(),
            PluginState::Registering
        );
        assert_eq!(
            serde_json::from_value::<PluginState>(json!({"kind":"registering"})).unwrap(),
            PluginState::Registering
        );
    }
}
