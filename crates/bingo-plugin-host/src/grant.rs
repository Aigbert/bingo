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
    let state: bingo_sdk::PluginState =
        serde_json::from_value(entry.meta.get("state").cloned().unwrap_or_default())
            .map_err(|_| format!("{plugin}: plugin has no admission state"))?;
    if !state.is_on() {
        return Err(format!(
            "{plugin}: {}",
            state.reason().unwrap_or("plugin is not enabled")
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
    input: &serde_json::Value,
) -> Result<(), String> {
    let face = wire.face();
    let noun = face.as_ref().map(|face| face.noun.as_str()).unwrap_or(key);
    let unsafe_call = face
        .as_ref()
        .and_then(|face| face.find(method))
        .is_none_or(|method| method.unsafe_call);
    check(host, plugin, noun, method, unsafe_call, input).await
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

/// The exact words are shared by both transports and dynamic filesystem grants.
pub fn refusal(
    plugin: &str,
    noun: &str,
    method: &str,
    reason: &str,
    settings: &std::path::Path,
) -> String {
    format!(
        "{plugin} may not {noun}.{method}: {reason}. Grant it under pluginTrust in {}.",
        settings.display()
    )
}

pub async fn check(
    host: &bingo_sdk::HostHandle,
    plugin: &str,
    noun: &str,
    method: &str,
    unsafe_call: bool,
    input: &serde_json::Value,
) -> Result<(), String> {
    let result = admits(&granted(host, plugin).await?, noun, method, unsafe_call);
    let audit = host.service_wire(AUDIT);
    if let Err(error) = result {
        let settings = match &audit {
            Some(audit) => audit
                .call("settings", serde_json::Value::Null)
                .await
                .ok()
                .and_then(|v| v.as_str().map(std::path::PathBuf::from)),
            None => None,
        }
        .unwrap_or_else(|| "<user settings path>".into());
        return Err(refusal(plugin, noun, method, &error.to_string(), &settings));
    }
    if unsafe_call || UNSAFE.contains(&noun) {
        tracing::info!(plugin, noun, method, session = ?input.get("session"), "granted unsafe plugin operation");
        if let Some(audit) = audit {
            let first = audit
                .call(
                    "record",
                    serde_json::json!({"plugin":plugin,"noun":noun,"session":input.get("session")}),
                )
                .await
                .map_err(|e| e.to_string())?;
            if first == true {
                host.notice(
                    bingo_sdk::Level::Info,
                    "PLUGIN_UNSAFE",
                    &format!("{plugin} uses the granted {noun} noun"),
                )
                .await
                .map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}
const AUDIT: &str = "bingo.plugin-host.grants";

pub struct Audit {
    settings: std::path::PathBuf,
    seen: std::sync::Mutex<BTreeSet<(String, String, Option<String>)>>,
}
impl Audit {
    pub fn install(
        host: &bingo_sdk::HostHandle,
        env: &bingo_sdk::Env,
    ) -> Result<std::sync::Arc<Self>, bingo_sdk::KernelError> {
        let audit = std::sync::Arc::new(Self {
            settings: env.user_settings(),
            seen: Default::default(),
        });
        host.open_service(AUDIT, audit.clone())?;
        Ok(audit)
    }
    pub fn end(&self, session: &bingo_sdk::SessionId) {
        self.seen
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|(_, _, id)| id.as_deref() != Some(session.as_str()));
    }
}
#[async_trait::async_trait]
impl bingo_sdk::WireService for Audit {
    async fn call(
        &self,
        method: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, bingo_sdk::ServiceError> {
        match method {
            "settings" => Ok(serde_json::json!(self.settings)),
            "record" => {
                let plugin = input["plugin"]
                    .as_str()
                    .ok_or_else(|| bingo_sdk::ServiceError("plugin is required".into()))?;
                let noun = input["noun"]
                    .as_str()
                    .ok_or_else(|| bingo_sdk::ServiceError("noun is required".into()))?;
                let session = input["session"].as_str().map(str::to_owned);
                Ok(serde_json::json!(
                    self.seen.lock().unwrap_or_else(|e| e.into_inner()).insert((
                        plugin.into(),
                        noun.into(),
                        session
                    ))
                ))
            }
            _ => Err(bingo_sdk::ServiceError(
                "unknown grant audit operation".into(),
            )),
        }
    }
}

#[cfg(test)]
mod world_tests {
    use super::*;
    use bingo_sdk::{Env, SessionId, WireService};
    use serde_json::json;
    #[test]
    fn refusal_words_name_the_operation_and_actual_settings() {
        assert_eq!(
            refusal(
                "sample",
                "fs",
                "write",
                "outside data",
                std::path::Path::new("settings.toml")
            ),
            "sample may not fs.write: outside data. Grant it under pluginTrust in settings.toml."
        );
    }
    #[test]
    fn every_unsafe_world_method_requires_a_user_grant_for_project_modules() {
        let uses = ["fs", "http", "process", "env", "mcp", "agent"]
            .map(str::to_owned)
            .to_vec();
        let project = grant(PluginRuntime::Module, Origin::Project, None, &uses);
        let trusted = grant(
            PluginRuntime::Module,
            Origin::Project,
            Some(&PluginTrust {
                tier: Tier::Rewrite,
                uses: uses.clone(),
            }),
            &uses,
        );
        for (noun, methods) in [
            ("fs", &["write"][..]),
            ("http", &["fetch"][..]),
            ("process", &["run", "spawn", "read", "write", "kill"][..]),
            ("env", &["get", "set"][..]),
            ("mcp", &["connect"][..]),
            ("agent", &["spawn"][..]),
        ] {
            for method in methods {
                assert!(
                    admits(&project, noun, method, true).is_err(),
                    "{noun}.{method}"
                );
                assert!(
                    admits(&trusted, noun, method, true).is_ok(),
                    "{noun}.{method}"
                );
            }
        }
    }
    #[tokio::test]
    async fn unsafe_nouns_are_recorded_once_per_plugin_and_session() {
        let host = bingo_sdk::testing::TestHost::default();
        let audit = Audit::install(&host.handle(), &Env::rooted("fixture")).unwrap();
        let session = SessionId::mint();
        let input = json!({"plugin":"one","noun":"process","session":session});
        assert_eq!(audit.call("record", input.clone()).await.unwrap(), true);
        assert_eq!(audit.call("record", input.clone()).await.unwrap(), false);
        assert_eq!(
            audit
                .call(
                    "record",
                    json!({"plugin":"two","noun":"process","session":session})
                )
                .await
                .unwrap(),
            true
        );
        assert_eq!(
            audit
                .call(
                    "record",
                    json!({"plugin":"one","noun":"http","session":session})
                )
                .await
                .unwrap(),
            true
        );
        audit.end(&session);
        assert_eq!(audit.call("record", input).await.unwrap(), true);
    }
}
