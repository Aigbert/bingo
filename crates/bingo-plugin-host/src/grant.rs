//! A directory and the person's explicit trust bound the requested nouns.
pub use bingo_sdk::{PluginGrant as Grant, PluginOrigin as Origin};
use bingo_sdk::{PluginRuntime, PluginTrust, Tier};
use std::collections::BTreeSet;

pub const UNSAFE: &[&str] = &["process", "http", "env", "model"];

pub fn grant(
    runtime: PluginRuntime,
    origin: Origin,
    trust: Option<&PluginTrust>,
    uses: &[String],
) -> Grant {
    let tier = trust.map_or(
        if runtime == PluginRuntime::Process || origin == Origin::Project {
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

/// Face metadata decides the noun and whether the method needs unsafe authority.
pub async fn admit_service(
    host: &bingo_sdk::HostHandle,
    plugin: &str,
    key: &str,
    method: &str,
    wire: &dyn bingo_sdk::WireService,
) -> Result<(), String> {
    let face = wire.face();
    let noun = face.as_ref().map(|face| face.noun.as_str()).unwrap_or(key);
    let unsafe_call = face
        .as_ref()
        .and_then(|face| face.find(method))
        .is_none_or(|method| method.unsafe_call);
    admits(&granted(host, plugin).await?, noun, method, unsafe_call)
        .map_err(|error| format!("{plugin}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_origin_and_user_trust_bound_grants() {
        use Origin::{Explicit, Project, User};
        use PluginRuntime::{Module, Process};
        use Tier::{Full, Rewrite, Tighten};

        let declared = vec!["session".into(), "model".into()];
        for (runtime, origin, default_tier, default_uses) in [
            (Process, User, Tighten, vec!["session", "model"]),
            (Process, Explicit, Tighten, vec!["session", "model"]),
            (Process, Project, Tighten, vec!["session"]),
            (Module, User, Rewrite, vec!["session", "model"]),
            (Module, Explicit, Rewrite, vec!["session", "model"]),
            (Module, Project, Tighten, vec!["session"]),
        ] {
            for tier in [None, Some(Tighten), Some(Rewrite), Some(Full)] {
                let trust = tier.map(|tier| PluginTrust {
                    tier,
                    uses: vec!["model".into(), "env".into()],
                });
                let granted = grant(runtime, origin, trust.as_ref(), &declared);
                let case = format!("{runtime:?} × {origin:?} × {tier:?}");
                assert_eq!(granted.tier, tier.unwrap_or(default_tier), "{case}");
                let expected_uses = if trust.is_some() {
                    vec!["session", "model", "env"]
                } else {
                    default_uses.clone()
                };
                assert_eq!(
                    granted.uses,
                    expected_uses.into_iter().map(str::to_owned).collect(),
                    "{case}"
                );
                assert!(
                    admits(&granted, "session", "extend", false).is_ok(),
                    "{case}"
                );
                assert_eq!(
                    admits(&granted, "model", "complete", true).is_ok(),
                    granted.tier != Tighten && granted.uses.contains("model"),
                    "{case}"
                );
                assert!(
                    admits(&granted, "undeclared", "read", false).is_err(),
                    "{case}"
                );
            }
        }
    }

    #[test]
    fn origin_and_explicit_trust_bound_unsafe_methods() {
        let uses = vec!["model".into(), "agents".into(), "state".into()];
        for origin in [Origin::User, Origin::Explicit] {
            let grant = grant(PluginRuntime::Module, origin, None, &uses);
            assert_eq!(grant.tier, Tier::Rewrite);
            assert!(admits(&grant, "model", "complete", false).is_ok());
        }
        let project = grant(PluginRuntime::Module, Origin::Project, None, &uses);
        assert!(admits(&project, "state", "get", false).is_ok());
        assert!(
            admits(&project, "model", "complete", false)
                .unwrap_err()
                .to_string()
                .contains("model grant")
        );
        assert!(admits(&project, "agents", "spawn", true).is_err());
        let trusted = grant(
            PluginRuntime::Module,
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
