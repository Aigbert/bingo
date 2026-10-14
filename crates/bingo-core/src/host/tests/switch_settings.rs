use super::*;

#[tokio::test]
async fn invalid_switch_does_not_migrate_the_user_document() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let env = env_in(home.path());
    let user = settings::user_path(&env);
    let legacy = user.with_extension("json");
    std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    let bytes = b"{\"enabledPlugins\":{\"test.provider\":true}}";
    std::fs::write(&legacy, bytes).unwrap();
    let path = project.path().join("settings.json");
    std::fs::write(&path, "{}").unwrap();
    let mut config = HostConfig::new(env.clone());
    config.layers = settings::load(&env, project.path(), Some(&path)).unwrap();
    let host = Host::build(vec![TestPlugin::boxed(&PROVIDER, vec![])], config)
        .await
        .unwrap();
    std::fs::write(&path, "{broken").unwrap();
    assert!(
        crate::plugins::switch_in(&host, &SessionId::mint(), PROVIDER.id, false)
            .await
            .is_err()
    );
    assert!(!user.exists(), "refusal migrated user settings");
    assert_eq!(std::fs::read(&legacy).unwrap(), bytes);
    assert!(!legacy.with_extension("json.bak").exists());
    host.shutdown().await;
}

#[tokio::test]
async fn deleted_settings_layer_does_not_keep_its_boot_override() {
    for explicit in [false, true] {
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let env = env_in(home.path());
        let path = project.path().join(if explicit {
            "settings.json"
        } else {
            ".bingo/settings.json"
        });
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{"enabledPlugins":{"test.provider":true}}"#).unwrap();
        let mut config = HostConfig::new(env.clone());
        config.layers =
            settings::load(&env, project.path(), explicit.then_some(path.as_path())).unwrap();
        let host = Host::build(vec![TestPlugin::boxed(&PROVIDER, vec![])], config)
            .await
            .unwrap();
        std::fs::remove_file(&path).unwrap();
        let result = crate::plugins::switch_in(&host, &SessionId::mint(), PROVIDER.id, false)
            .await
            .unwrap();
        assert!(
            matches!(result, crate::plugins::Switched::Applied(false, _)),
            "deleted override survived"
        );
        assert!(
            host.registry
                .switches
                .wishes
                .read()
                .unwrap()
                .contains(PROVIDER.id)
        );
        host.shutdown().await;
    }
}
