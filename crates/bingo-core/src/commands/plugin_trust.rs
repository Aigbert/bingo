//! Person-owned grants written through the same command path as panel keys.
use crate::{host::Host, settings};
use bingo_sdk::*;
use serde_json::json;

pub(super) async fn set(host: &Host, name: &str, tier: &str) -> Result<(), KernelError> {
    if !render::asked() {
        return Err(KernelError::new(
            ErrorCode::PermissionDenied,
            "trust requires person input",
        ));
    }
    let tier: Tier = serde_json::from_value(json!(tier)).map_err(|_| {
        KernelError::new(ErrorCode::InvalidInput, "trust is tighten, rewrite or full")
    })?;
    let listed = crate::plugins::listing(host.registry()).await;
    let status = listed_as(&listed, name)
        .ok_or_else(|| KernelError::new(ErrorCode::InvalidInput, "no plugin with that name"))?;
    if status.from == BUILT_IN || name.starts_with("mcp:") {
        return Err(KernelError::new(
            ErrorCode::InvalidInput,
            "only external plugins have a configurable trust tier",
        ));
    }
    let _guard = host.registry().switches.switching.lock().await;
    let path = settings::user_path(host.env());
    let mut document = settings::preview_document(&path).map_err(failed)?;
    let grants = document.entry("pluginTrust").or_insert_with(|| json!({}));
    let grants = grants.as_object_mut().ok_or_else(|| {
        KernelError::new(ErrorCode::InvalidInput, "pluginTrust must be an object")
    })?;
    let name = name.strip_prefix("plugins:").unwrap_or(name);
    let grant = grants.entry(name).or_insert_with(|| json!({}));
    if !grant.is_object() {
        return Err(KernelError::new(
            ErrorCode::InvalidInput,
            "plugin grant must be an object",
        ));
    }
    grant["tier"] = json!(tier);
    settings::remember(&path, &[("pluginTrust", document["pluginTrust"].clone())])
        .map_err(failed)?;
    host.plugins_changed();
    Ok(())
}
/// A plugin's listing by its exact id first, then by the same name with or
/// without the `plugins:` a repository's plugin carries while it waits.
fn listed_as<'a>(listed: &'a [PluginStatus], name: &str) -> Option<&'a PluginStatus> {
    let bare = name.strip_prefix("plugins:").unwrap_or(name);
    listed.iter().find(|entry| entry.id == name).or_else(|| {
        listed
            .iter()
            .find(|entry| entry.id == bare || entry.id.strip_prefix("plugins:") == Some(bare))
    })
}
fn failed(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::Internal, error.to_string())
}
pub(super) async fn run(host: &Host, args: &str) -> Result<CommandOutcome, KernelError> {
    let words = args.split_whitespace().collect::<Vec<_>>();
    let [name, tier] = words.as_slice() else {
        return Err(KernelError::new(
            ErrorCode::InvalidInput,
            "usage: /plugins trust <name> <tighten|rewrite|full>",
        ));
    };
    set(host, name, tier).await?;
    let when = reapplied(host, name).await;
    host.plugins_changed();
    let when = when.map_err(|error| {
        KernelError::new(
            error.code,
            format!(
                "{name}: {tier} kept in ~/.bingo/settings.toml, but it did not come back on: {}; \
                 /plugins enable {name} turns it on",
                error.message
            ),
        )
    })?;
    Ok(CommandOutcome::Applied {
        message: Some(format!(
            "{name}: {tier} kept in ~/.bingo/settings.toml; applies {when}"
        )),
    })
}

/// A running plugin takes a new grant by going off and on again where its
/// source can do that now; anything else takes it at next start.
async fn reapplied(host: &Host, name: &str) -> Result<&'static str, KernelError> {
    let status = crate::plugins::listing(host.registry())
        .await
        .into_iter()
        .find(|p| p.id == name);
    if !status.is_some_and(|p| p.state.is_on()) {
        return Ok("at next start");
    }
    for source in &host.registry().sources.plugins {
        if source.switch_kind(name).await == plugin::PluginSwitch::Now {
            source.switch(name, false).await?;
            return Ok(match source.switch(name, true).await? {
                plugin::PluginSwitch::Now => "now",
                plugin::PluginSwitch::AfterTurn => "after the running turn",
                plugin::PluginSwitch::NextStart { .. } => "at next start",
            });
        }
    }
    Ok("at next start")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    struct Source(
        Mutex<Vec<plugin::PluginSwitch>>,
        Mutex<Option<Tier>>,
        std::sync::atomic::AtomicBool,
    );
    #[async_trait::async_trait]
    impl PluginSource for Source {
        fn id(&self) -> &str {
            "fixture"
        }
        async fn plugins(&self) -> Vec<PluginStatus> {
            vec![PluginStatus {
                id: "outside".into(),
                version: "1".into(),
                state: PluginState::On,
                from: "fixture".into(),
                placement: Some(plugin::PluginPlacement {
                    origin: plugin::PluginOrigin::User,
                    runtime: plugin::PluginRuntime::Module,
                }),
                grant: Some(PluginGrant {
                    tier: Tier::Tighten,
                    uses: Default::default(),
                }),
            }]
        }
        async fn switch_kind(&self, _: &str) -> plugin::PluginSwitch {
            plugin::PluginSwitch::Now
        }
        async fn switch(
            &self,
            _: &str,
            enabled: bool,
        ) -> Result<plugin::PluginSwitch, KernelError> {
            if enabled && self.2.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(KernelError::new(ErrorCode::Internal, "the module threw"));
            }
            self.0.lock().unwrap().push(if enabled {
                plugin::PluginSwitch::Now
            } else {
                plugin::PluginSwitch::AfterTurn
            });
            Ok(plugin::PluginSwitch::Now)
        }
        async fn registrations(
            &self,
            trust: &std::collections::BTreeMap<String, PluginTrust>,
        ) -> Vec<plugin::PluginRegistration> {
            *self.1.lock().unwrap() = trust.get("outside").map(|grant| grant.tier);
            vec![]
        }
    }
    struct Fixture(Arc<Source>);
    impl Plugin for Fixture {
        fn manifest(&self) -> &'static PluginManifest {
            static M: PluginManifest = PluginManifest {
                id: "fixture",
                version: "1",
                uses: &[],
                requires: &[],
                provides: &[],
                config: None,
            };
            &M
        }
        fn register(&self, r: &mut Registrar) -> Result<(), PluginError> {
            r.add(Contribution::Plugins(self.0.clone()));
            r.add(Contribution::Provider(
                crate::test_support::ScriptedProvider::new(vec![]),
            ));
            Ok(())
        }
    }
    #[tokio::test]
    async fn only_person_input_saves_a_grant_and_new_registration_reads_it() {
        let home = tempfile::tempdir().unwrap();
        let source = Arc::new(Source(
            Default::default(),
            Default::default(),
            Default::default(),
        ));
        let host = Host::build(
            vec![Box::new(Fixture(source.clone()))],
            crate::HostConfig::new(Env::rooted(home.path())),
        )
        .await
        .unwrap();
        let path = settings::user_path(host.env());
        assert!(set(&host, "outside", "full").await.is_err());
        assert!(!path.exists());
        assert!(
            render::under_input(true, set(&host, "outside", "unknown"))
                .await
                .is_err()
        );
        assert!(!path.exists());
        settings::remember(
            &path,
            &[(
                "pluginTrust",
                json!({"outside":{"tier":"tighten","uses":["fs"]},"other":{"tier":"rewrite"}}),
            )],
        )
        .unwrap();
        render::under_input(true, run(&host, "outside full"))
            .await
            .unwrap();
        let saved = settings::read_document(&path).unwrap();
        assert_eq!(saved["pluginTrust"]["outside"]["tier"], "full");
        assert_eq!(saved["pluginTrust"]["outside"]["uses"], json!(["fs"]));
        assert_eq!(saved["pluginTrust"]["other"]["tier"], "rewrite");
        HostApi::catalog(host.as_ref(), CatalogKind::Plugins)
            .await
            .unwrap();
        assert_eq!(*source.1.lock().unwrap(), Some(Tier::Full));
        assert_eq!(source.0.lock().unwrap().len(), 2);
        std::fs::write(&path, "[invalid").unwrap();
        HostApi::catalog(host.as_ref(), CatalogKind::Plugins)
            .await
            .unwrap();
        assert_eq!(
            *source.1.lock().unwrap(),
            Some(Tier::Full),
            "invalid live settings must not start a registration with default grants"
        );
        host.shutdown().await;
    }

    /// An unreadable trust file keeps waiting plugins off and says which
    /// file, once, to the sessions that are open.
    #[tokio::test]
    async fn an_unreadable_trust_file_is_named_once() {
        let home = tempfile::tempdir().unwrap();
        let source = Arc::new(Source(
            Default::default(),
            Default::default(),
            Default::default(),
        ));
        let host = Host::build(
            vec![Box::new(Fixture(source.clone()))],
            crate::HostConfig::new(Env::rooted(home.path()))
                .with_layer("cli", json!({"model":"m"})),
        )
        .await
        .unwrap();
        let open = |id: Option<SessionId>| {
            let selector = match id {
                Some(id) => SessionSelector::ById { id },
                None => SessionSelector::Create {
                    spec: SessionSpec {
                        cwd: home.path().to_path_buf(),
                        ..Default::default()
                    },
                },
            };
            let who = ClientIdentity {
                name: "test".into(),
                surface: "test".into(),
            };
            HostApi::open(host.as_ref(), selector, who, OpenOptions::default())
        };
        let session = open(None).await.unwrap();
        let path = settings::user_path(host.env());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "[invalid").unwrap();
        for _ in 0..3 {
            HostApi::catalog(host.as_ref(), CatalogKind::Plugins)
                .await
                .unwrap();
        }
        assert_eq!(*source.1.lock().unwrap(), None, "nothing registered");
        let again = open(Some(session.snapshot.summary.id.clone()))
            .await
            .unwrap();
        let told = again
            .snapshot
            .transcript
            .items()
            .iter()
            .filter_map(|item| match &item.body {
                ItemBody::Notice { code, text, .. } if code == "PLUGIN_TRUST_UNREAD" => {
                    Some(text.clone())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(told.len(), 1, "{told:?}");
        assert!(told[0].contains(&path.display().to_string()), "{told:?}");
        drop((session, again));
        host.shutdown().await;
    }

    /// A grant that took but a plugin that did not come back on says both,
    /// and the one command that turns it on.
    #[tokio::test]
    async fn a_plugin_that_does_not_come_back_on_says_how_to_turn_it_on() {
        let home = tempfile::tempdir().unwrap();
        let source = Arc::new(Source(
            Default::default(),
            Default::default(),
            Default::default(),
        ));
        source.2.store(true, std::sync::atomic::Ordering::SeqCst);
        let host = Host::build(
            vec![Box::new(Fixture(source.clone()))],
            crate::HostConfig::new(Env::rooted(home.path())),
        )
        .await
        .unwrap();
        let error = render::under_input(true, run(&host, "outside rewrite"))
            .await
            .unwrap_err();
        assert!(error.message.contains("rewrite kept"), "{}", error.message);
        assert!(
            error.message.contains("the module threw"),
            "{}",
            error.message
        );
        assert!(
            error.message.contains("/plugins enable outside"),
            "{}",
            error.message
        );
        let saved = settings::read_document(&settings::user_path(host.env())).unwrap();
        assert_eq!(saved["pluginTrust"]["outside"]["tier"], "rewrite");
        host.shutdown().await;
    }
}
