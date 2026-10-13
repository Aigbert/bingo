//! Context and catalogue reads shared by interception sites.
use super::*;

pub(super) fn hook_set(host: &Host) -> HookSet {
    HookSet {
        links: host.registry.links.clone(),
        interceptors: host
            .registry
            .sources
            .interceptors
            .iter()
            .map(|source| {
                Arc::new(RegisteredSource {
                    host: host.weak.clone(),
                    source: source.clone(),
                }) as Arc<dyn InterceptorSource>
            })
            .collect(),
        grants: host.settings.kernel.plugin_trust.clone(),
        fixed: host.registry.hooks.clone(),
        sources: host.registry.sources.hooks.clone(),
    }
}

pub(super) fn catalogue_context(host: &Host, session: Option<&SessionId>) -> HookContext {
    let cwd = session
        .and_then(|id| host.live(id).ok())
        .map(|live| live.spec.cwd)
        .unwrap_or_else(|| host.config.env.home.clone());
    HookContext {
        session: session
            .cloned()
            .unwrap_or_else(|| SessionId::from_raw("host")),
        turn: None,
        item: None,
        cwd,
        provider: None,
        model: None,
        host: host.handle(),
    }
}

pub(super) async fn catalogue_answer(
    host: &Host,
    kind: CatalogKind,
) -> Result<Catalog, KernelError> {
    register_pending(host).await;
    let resolved = host.providers().await;
    let catalog = Catalog {
        kind,
        entries: catalog::entries(
            &host.registry,
            &resolved,
            host.settings.kernel.model.as_deref(),
            &host.served,
            kind,
        )
        .await,
    };
    let report = crate::chain::Report::default();
    let result = crate::chain::autocomplete::catalog(
        catalog,
        &host.hook_set().links().await,
        &host.catalogue_context(None),
        report.clone(),
    )
    .await;
    for notice in report.take() {
        if let ItemBody::Notice { level, code, text } = notice {
            let _ = host.notice(level, &code, &text).await;
        }
    }
    result
}

struct RegisteredSource {
    host: Weak<Host>,
    source: Arc<dyn InterceptorSource>,
}
#[async_trait]
impl InterceptorSource for RegisteredSource {
    fn id(&self) -> &str {
        self.source.id()
    }
    async fn interceptors(&self) -> Vec<ExternalInterceptor> {
        if let Some(host) = self.host.upgrade() {
            register_pending(&host).await;
        }
        self.source.interceptors().await
    }
    async fn tier(&self, name: &str) -> Option<Tier> {
        let host = self.host.upgrade()?;
        crate::plugins::listing(&host.registry)
            .await
            .into_iter()
            .find(|status| status.id == name)?
            .execution
            .map(|execution| execution.grant.tier)
    }
}

pub(super) async fn register_pending(host: &Host) {
    for source in &host.registry.sources.plugins {
        for registration in source
            .registrations(&host.settings.kernel.plugin_trust)
            .await
        {
            let name = registration.name.clone();
            let links = registration_links(host).await;
            let result = crate::chain::registration::register(
                registration,
                &links,
                &host.catalogue_context(None),
            )
            .await;
            source.registered(&name, result).await;
        }
    }
}

async fn registration_links(host: &Host) -> Vec<crate::chain::Link> {
    let mut links = host.registry.links.clone();
    let statuses = crate::plugins::listing(&host.registry).await;
    for source in &host.registry.sources.interceptors {
        for external in source.interceptors().await {
            let status = statuses.iter().find(|status| status.id == external.plugin);
            if status.is_some_and(|status| !status.enabled) {
                continue;
            }
            let tier = status
                .and_then(|status| status.execution.as_ref())
                .map(|execution| execution.grant.tier)
                .or_else(|| {
                    host.settings
                        .kernel
                        .plugin_trust
                        .get(&external.plugin)
                        .map(|trust| trust.tier)
                })
                .unwrap_or(Tier::Tighten);
            links.push(crate::chain::Link {
                external: Some(external.plugin),
                interceptor: external.link,
                tier,
            });
        }
    }
    links
}
