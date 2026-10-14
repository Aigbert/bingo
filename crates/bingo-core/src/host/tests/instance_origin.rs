use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

struct Changing {
    retired: AtomicBool,
    origin: String,
    fail_logout: bool,
    spoil: Option<std::path::PathBuf>,
}
#[async_trait]
impl Provider for Changing {
    fn id(&self) -> &str {
        "work"
    }
    fn instance(&self) -> bool {
        true
    }
    fn shape(&self) -> Option<&str> {
        Some("local")
    }
    fn credential_origin(&self) -> Option<String> {
        Some(self.origin.clone())
    }
    fn retire(&self, _: &str) {
        self.retired.store(true, Ordering::Relaxed);
    }
    fn readdress(&self, spec: &InstanceSpec) -> Result<Arc<dyn Provider>, PluginError> {
        Ok(Arc::new(Self {
            retired: AtomicBool::new(false),
            origin: spec.base_url.clone().unwrap(),
            fail_logout: false,
            spoil: None,
        }))
    }
    fn endpoint(&self, _: &str) -> EndpointCapabilities {
        EndpointCapabilities::default()
    }
    async fn stream(
        &self,
        _: ModelRequest,
        _: CancellationToken,
    ) -> Result<ModelStream, ProviderError> {
        Err(ProviderError::Timeout)
    }
    async fn logout(&self) -> Result<String, ProviderError> {
        if self.fail_logout {
            return Err(ProviderError::Timeout);
        }
        if let Some(path) = &self.spoil {
            std::fs::remove_file(path).unwrap();
            std::fs::create_dir(path).unwrap();
        }
        Ok("forgotten".into())
    }
}

async fn changing(
    dir: &std::path::Path,
    fail_logout: bool,
    spoil: bool,
    layer: Option<settings::Layer>,
) -> (Arc<Host>, Arc<Changing>) {
    let env = Env::rooted(dir);
    let path = settings::user_path(&env);
    settings::instances::add(
        &path,
        "local",
        "work",
        &InstanceSpec {
            base_url: Some("http://old".into()),
            images: None,
        },
    )
    .unwrap();
    let provider = Arc::new(Changing {
        retired: AtomicBool::new(false),
        origin: "http://old".into(),
        fail_logout,
        spoil: spoil.then_some(path),
    });
    let mut config = HostConfig::new(env);
    config.layers.extend(layer);
    let host = Host::build(
        vec![TestPlugin::boxed(
            &PROVIDER,
            vec![
                Contribution::Provider(provider.clone()),
                Contribution::ProviderShape(Arc::new(super::instances::Shape)),
            ],
        )],
        config,
    )
    .await
    .unwrap();
    (host, provider)
}

#[tokio::test]
async fn failed_origin_writes_do_not_retire_the_current_provider() {
    for (logout, write) in [(true, false), (false, true)] {
        let dir = tempfile::tempdir().unwrap();
        let (host, old) = changing(dir.path(), logout, write, None).await;
        assert!(
            provider::set_provider(&host, "work", "http://new")
                .await
                .is_err()
        );
        assert!(!old.retired.load(Ordering::Relaxed));
        assert_eq!(
            host.provider(Some("work"))
                .await
                .unwrap()
                .credential_origin()
                .as_deref(),
            Some("http://old")
        );
    }
}
#[tokio::test]
async fn origin_change_refuses_keys_in_every_non_user_layer() {
    for origin in [
        settings::LayerOrigin::Repository("/repo".into()),
        settings::LayerOrigin::Explicit,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let layer = settings::Layer::new(
            origin,
            "/repo/settings.local.toml",
            json!({
                "local": {"instances": {"work": {"apiKey":"DO-NOT-ECHO"}}}
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        let (host, old) = changing(dir.path(), false, false, Some(layer)).await;
        let error = provider::set_provider(&host, "work", "http://new")
            .await
            .err()
            .unwrap();
        assert!(
            error.message.contains("/repo/settings.local.toml"),
            "{error:?}"
        );
        assert!(!error.message.contains("DO-NOT-ECHO"));
        assert!(!old.retired.load(Ordering::Relaxed));
        assert_eq!(
            settings::read_document(&settings::user_path(host.env())).unwrap()["local"]["instances"]
                ["work"]["baseUrl"],
            "http://old"
        );
    }
}
