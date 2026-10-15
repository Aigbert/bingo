//! A mod may not register under a name a built-in already answers to.

use super::*;
use std::sync::Mutex;

/// A source asking to register every name it was given, writing down what
/// the host answered for each.
struct Asking {
    names: Vec<&'static str>,
    answers: Mutex<Vec<(String, Result<PluginGrant, String>)>>,
}
#[async_trait]
impl bingo_sdk::PluginSource for Asking {
    fn id(&self) -> &str {
        "asking"
    }
    async fn plugins(&self) -> Vec<PluginStatus> {
        Vec::new()
    }
    async fn registrations(
        &self,
        _: &std::collections::BTreeMap<String, bingo_sdk::PluginTrust>,
    ) -> Vec<bingo_sdk::plugin::PluginRegistration> {
        self.names
            .iter()
            .map(|name| bingo_sdk::plugin::PluginRegistration {
                name: (*name).into(),
                version: "1".into(),
                execution: bingo_sdk::plugin::PluginExecution {
                    origin: PluginOrigin::User,
                    runtime: PluginRuntime::Module,
                    grant: PluginGrant {
                        tier: Tier::Tighten,
                        uses: Default::default(),
                    },
                },
            })
            .collect()
    }
    async fn registered(&self, name: &str, result: Result<PluginGrant, String>) {
        self.answers.lock().unwrap().push((name.into(), result));
    }
}

#[tokio::test]
async fn a_mod_cannot_take_a_built_ins_name_or_handler() {
    let home = tempfile::tempdir().unwrap();
    let source = Arc::new(Asking {
        names: vec!["test.provider", "palette", "fresh"],
        answers: Mutex::default(),
    });
    let handler = Contribution::Command(ScriptedCommand::new(
        "palette:ui",
        true,
        Ok(CommandOutcome::Applied { message: None }),
    ));
    let _host = Host::build(
        vec![TestPlugin::boxed(
            &PROVIDER,
            vec![
                Contribution::Provider(ScriptedProvider::new(vec![])),
                Contribution::Plugins(source.clone()),
                handler,
            ],
        )],
        HostConfig::new(env_in(home.path())).with_layer("cli", json!({"model":"m"})),
    )
    .await
    .unwrap();
    let answers = source.answers.lock().unwrap();
    let refused: Vec<_> = answers
        .iter()
        .filter(|(_, result)| result.is_err())
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(refused, ["test.provider", "palette"], "{answers:?}");
    assert!(
        answers
            .iter()
            .any(|(name, result)| name == "fresh" && result.is_ok()),
        "{answers:?}"
    );
}
