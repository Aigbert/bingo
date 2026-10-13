//! Configured channel credentials.

use crate::feishu::Feishu;
use crate::loopback::Loopback;
use crate::settings::{APP_SECRET, Settings};
pub use bingo_channel_adapter::secret::*;
use serde_json::Value;

/// Every adapter this build has, and what each signs with. The one place
/// those two facts live together; everything below is a filter of it.
fn adapters() -> [Requirement; 2] {
    [
        Requirement {
            // A loopback dials a peer on this machine; there is nothing to sign.
            id: Loopback::ID,
            variable: None,
        },
        Requirement {
            id: Feishu::ID,
            variable: Some(APP_SECRET),
        },
    ]
}

/// The adapters a secret can be stored for: what `bingo channels secret`
/// accepts, so a typo is refused rather than written down.
pub fn signing() -> Vec<Requirement> {
    adapters()
        .into_iter()
        .filter(|wanted| wanted.variable.is_some())
        .collect()
}

/// Every adapter a merged settings object asks for, with its credential.
///
/// The bin asks this so `gateway doctor` can name a channel's credential
/// without the bin ever learning how a channel is spelled — the same reason
/// [`crate::wanted`] exists, and now the same answer behind both.
pub fn configured(settings: &Value) -> Vec<Requirement> {
    let Ok(settings) = serde_json::from_value::<Settings>(settings.clone()) else {
        return Vec::new();
    };
    adapters()
        .into_iter()
        .filter(|wanted| match wanted.id {
            Loopback::ID => settings.channels.loopback.is_some(),
            Feishu::ID => settings.channels.feishu.is_some(),
            _ => false,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn only_the_channels_the_settings_name_are_asked_for_and_only_feishu_signs() {
        assert!(configured(&json!({})).is_empty());
        assert_eq!(
            configured(&json!({ "channels": { "loopback": {} } })),
            [Requirement {
                id: "loopback",
                variable: None
            }]
        );
        assert_eq!(
            configured(&json!({ "channels": { "feishu": { "appId": "cli_a" } } })),
            [Requirement {
                id: "feishu",
                variable: Some(APP_SECRET)
            }]
        );
        assert_eq!(
            configured(&json!({ "channels": { "loopback": {}, "feishu": {} } })).len(),
            2
        );
        assert!(
            configured(&json!({ "channels": "nonsense" })).is_empty(),
            "settings that will not parse ask for nothing"
        );
    }
}
