//! Context and catalogue reads shared by interception sites.
use super::*;

pub(super) fn hook_set(host: &Host) -> HookSet {
    HookSet {
        links: host.registry.links.clone(),
        interceptors: host.registry.sources.interceptors.clone(),
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
