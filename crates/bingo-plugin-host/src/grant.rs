//! A directory and the person's explicit trust bound the requested nouns.
pub use bingo_sdk::{PluginGrant as Grant, PluginOrigin as Origin};
use bingo_sdk::{PluginTrust, Tier};
use std::collections::BTreeSet;

pub const UNSAFE: &[&str] = &["process", "http", "env", "model"];

pub fn grant(origin: Origin, trust: Option<&PluginTrust>, uses: &[String]) -> Grant {
    let tier = trust.map_or(
        if origin == Origin::Project {
            Tier::Tighten
        } else {
            Tier::Rewrite
        },
        |trust| trust.tier,
    );
    let mut uses: BTreeSet<String> = uses
        .iter()
        .filter(|noun| origin != Origin::Project || !UNSAFE.contains(&noun.as_str()))
        .cloned()
        .collect();
    if let Some(trust) = trust {
        uses.extend(trust.uses.iter().cloned());
    }
    Grant { tier, uses }
}

#[derive(Debug, thiserror::Error)]
#[error("{noun}.{method} needs the {noun} grant{reason}")]
pub struct Refused {
    pub noun: String,
    pub method: String,
    reason: &'static str,
}

pub fn admits(grant: &Grant, noun: &str, method: &str, unsafe_call: bool) -> Result<(), Refused> {
    let unsafe_call = unsafe_call || UNSAFE.contains(&noun);
    if !grant.uses.contains(noun) || (unsafe_call && grant.tier == Tier::Tighten) {
        return Err(Refused {
            noun: noun.into(),
            method: method.into(),
            reason: if unsafe_call {
                " for an unsafe operation"
            } else {
                ""
            },
        });
    }
    Ok(())
}

/// Read the kernel-adjudicated grant; display hooks cannot replace this metadata.
pub async fn granted(host: &bingo_sdk::HostHandle, plugin: &str) -> Result<Grant, String> {
    let catalog = host
        .catalog(bingo_sdk::CatalogKind::Plugins)
        .await
        .map_err(|error| error.message)?;
    let entry = catalog
        .entries
        .into_iter()
        .find(|entry| entry.id == plugin)
        .ok_or_else(|| format!("{plugin}: plugin is not registered"))?;
    if entry
        .meta
        .get("enabled")
        .and_then(serde_json::Value::as_bool)
        != Some(true)
    {
        return Err(format!(
            "{plugin}: {}",
            entry
                .meta
                .get("reason")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("plugin is not enabled")
        ));
    }
    serde_json::from_value(entry.meta).map_err(|_| format!("{plugin}: plugin has no granted nouns"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn origin_and_explicit_trust_bound_unsafe_methods() {
        let uses = vec!["model".into(), "agents".into(), "state".into()];
        for origin in [Origin::User, Origin::Explicit] {
            let grant = grant(origin, None, &uses);
            assert_eq!(grant.tier, Tier::Rewrite);
            assert!(admits(&grant, "model", "complete", false).is_ok());
        }
        let project = grant(Origin::Project, None, &uses);
        assert!(admits(&project, "state", "get", false).is_ok());
        assert!(
            admits(&project, "model", "complete", false)
                .unwrap_err()
                .to_string()
                .contains("model grant")
        );
        assert!(admits(&project, "agents", "spawn", true).is_err());
        let trusted = grant(
            Origin::Project,
            Some(&PluginTrust {
                tier: Tier::Full,
                uses,
            }),
            &[],
        );
        assert!(admits(&trusted, "agents", "spawn", true).is_ok());
        assert!(admits(&trusted, "undeclared", "read", false).is_err());
    }
}
