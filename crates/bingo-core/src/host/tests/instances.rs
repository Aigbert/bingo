use super::*;

struct Shape;
impl ProviderShape for Shape {
    fn id(&self) -> &str {
        "local"
    }
    fn credential_exists(&self, name: &str) -> Result<bool, PluginError> {
        Ok(name == "occupied")
    }
    fn takes_key(&self) -> bool {
        false
    }
    fn settings_key(&self) -> &str {
        "local"
    }
    fn build(&self, name: &str, spec: &InstanceSpec) -> Result<Arc<dyn Provider>, PluginError> {
        Ok(Arc::new(Local {
            name: name.into(),
            url: spec.base_url.clone().unwrap_or_default(),
        }))
    }
}
struct Local {
    name: String,
    url: String,
}
#[async_trait]
impl Provider for Local {
    fn id(&self) -> &str {
        &self.name
    }
    fn readdress(&self, spec: &InstanceSpec) -> Result<Arc<dyn Provider>, PluginError> {
        Shape.build(&self.name, spec)
    }
    fn shape(&self) -> Option<&str> {
        Some("local")
    }
    fn address(&self) -> Option<&str> {
        Some(&self.url)
    }
    fn instance(&self) -> bool {
        true
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
    async fn models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        if self.url.contains("down") {
            return Err(ProviderError::Transport {
                message: "https://user:secret@host?key=secret\nAuthorization: secret".into(),
            });
        }
        Ok(vec![ModelInfo {
            id: "m".into(),
            display: None,
        }])
    }
    async fn logout(&self) -> Result<String, ProviderError> {
        Ok("forgotten".into())
    }
}

async fn build(dir: &std::path::Path, shapes: usize) -> Result<Arc<Host>, HostError> {
    let contributions = (0..shapes)
        .map(|_| Contribution::ProviderShape(Arc::new(Shape)))
        .collect();
    Host::build(
        vec![TestPlugin::boxed(&PROVIDER, contributions)],
        HostConfig::new(Env::rooted(dir)),
    )
    .await
}

#[tokio::test]
async fn shape_contract_and_duplicate_registration() {
    let dir = tempfile::tempdir().unwrap();
    assert!(build(dir.path(), 2).await.is_err());
    let host = build(dir.path(), 1).await.unwrap();
    let (held, _, result) = provider::add_provider(&host, "work", "local", "http://up")
        .await
        .unwrap();
    assert_eq!(result.answer, Ok(1));
    assert!(
        provider::add_provider(&host, "work", "local", "http://up")
            .await
            .is_err()
    );
    assert!(
        provider::add_provider(&host, "local", "local", "http://up")
            .await
            .is_err()
    );
    let entries = host.catalog(CatalogKind::Providers).await.unwrap().entries;
    let meta = &entries[0].meta;
    let mut keys: Vec<_> = meta
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort();
    assert_eq!(keys, ["address", "auth", "instance", "reach", "shape"]);
    assert_eq!(meta["reach"]["kind"], "ok");
    provider::set_provider(&host, "work", "http://down")
        .await
        .unwrap();
    assert_eq!(held.address(), Some("http://up"));
    let reach = provider::provider_reach(&host, "work");
    assert_eq!(reach["kind"], "down");
    assert!(!reach.to_string().contains("secret"));
    provider::set_provider(&host, "work", "http://fixed")
        .await
        .unwrap();
    assert_eq!(provider::provider_reach(&host, "work")["kind"], "ok");
    provider::remove_provider(&host, "work").await.unwrap();
    assert!(host.providers().await.is_empty());
    assert!(host.served.get("work").is_none());
    assert_eq!(held.address(), Some("http://up"));
}

#[tokio::test]
async fn a_removed_or_replaced_provider_cannot_publish_a_late_probe() {
    let dir = tempfile::tempdir().unwrap();
    let host = build(dir.path(), 1).await.unwrap();
    let (old, _, _) = provider::add_provider(&host, "work", "local", "http://old")
        .await
        .unwrap();
    provider::set_provider(&host, "work", "http://down")
        .await
        .unwrap();
    refresh::one(&host, old.clone()).await;
    assert_eq!(provider::provider_reach(&host, "work")["kind"], "down");
    assert!(host.served.get("work").is_none());
    provider::remove_provider(&host, "work").await.unwrap();
    refresh::one(&host, old).await;
    assert_eq!(provider::provider_reach(&host, "work")["kind"], "unchecked");
    assert!(host.served.get("work").is_none());
}

#[tokio::test]
async fn unregistered_family_names_are_not_reserved_by_the_kernel() {
    let dir = tempfile::tempdir().unwrap();
    let host = build(dir.path(), 1).await.unwrap();
    assert!(
        provider::add_provider(&host, "fake", "local", "http://up")
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn adding_cannot_adopt_an_existing_credential() {
    let dir = tempfile::tempdir().unwrap();
    let host = build(dir.path(), 1).await.unwrap();
    let error = provider::add_provider(&host, "occupied", "local", "http://up")
        .await
        .err()
        .unwrap();
    assert!(error.message.contains("remove it first"));
    assert!(host.providers().await.is_empty());
    assert!(!crate::settings::user_path(host.env()).exists());
}

struct FreshProviders;
#[async_trait]
impl ProviderSource for FreshProviders {
    fn id(&self) -> &str {
        "fresh"
    }
    async fn providers(&self) -> Vec<Arc<dyn Provider>> {
        vec![Arc::new(Local {
            name: "remote".into(),
            url: "http://up".into(),
        })]
    }
}

#[tokio::test]
async fn a_source_that_rebuilds_arcs_can_publish_models_and_reach() {
    let dir = tempfile::tempdir().unwrap();
    let host = Host::build(
        vec![TestPlugin::boxed(
            &PROVIDER,
            vec![Contribution::Providers(Arc::new(FreshProviders))],
        )],
        HostConfig::new(Env::rooted(dir.path())),
    )
    .await
    .unwrap();
    let mut events = host.gateway.subscribe();
    host.refresh_models().await;
    assert_eq!(provider::provider_reach(&host, "remote")["kind"], "ok");
    assert!(host.served.get("remote").is_some());
    let mut models_changed = false;
    while let Ok(event) = events.try_recv() {
        models_changed |= matches!(
            event,
            GatewayEvent::CatalogChanged {
                kind: CatalogKind::Models
            }
        );
    }
    assert!(models_changed);
}

#[tokio::test]
async fn removal_clears_live_pins_before_another_pin_write() {
    let dir = tempfile::tempdir().unwrap();
    let host = build(dir.path(), 1).await.unwrap();
    provider::add_provider(&host, "work", "local", "http://up")
        .await
        .unwrap();
    pins::pin_model(&host, "work/m", true).unwrap();
    pins::pin_model(&host, "other/m", true).unwrap();
    provider::remove_provider(&host, "work").await.unwrap();
    assert_eq!(pins::pinned_models(&host), ["other/m"]);
    pins::pin_model(&host, "third/m", true).unwrap();
    let document = crate::settings::read_document(&crate::settings::user_path(host.env())).unwrap();
    assert_eq!(document["pinned"], json!(["other/m", "third/m"]));
}

#[tokio::test]
async fn readding_a_name_does_not_restore_its_cleared_default() {
    let dir = tempfile::tempdir().unwrap();
    let env = Env::rooted(dir.path());
    crate::settings::remember(
        &crate::settings::user_path(&env),
        &[("provider", json!("work"))],
    )
    .unwrap();
    let host = Host::build(
        vec![TestPlugin::boxed(
            &PROVIDER,
            vec![Contribution::ProviderShape(Arc::new(Shape))],
        )],
        HostConfig::new(env).with_layer("user", json!({"provider":"work"})),
    )
    .await
    .unwrap();
    provider::add_provider(&host, "work", "local", "http://up")
        .await
        .unwrap();
    provider::add_provider(&host, "other", "local", "http://up")
        .await
        .unwrap();
    provider::remove_provider(&host, "work").await.unwrap();
    assert_eq!(host.provider(None).await.unwrap().id(), "other");
    provider::add_provider(&host, "work", "local", "http://up")
        .await
        .unwrap();
    assert_eq!(host.provider(None).await.unwrap().id(), "other");
}
