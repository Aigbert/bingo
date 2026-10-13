//! Catalogue reads pass through description links before completion links.
use super::*;

pub(crate) async fn catalog(
    mut catalog: Catalog,
    links: &[Link],
    cx: &HookContext,
    report: Report,
) -> Result<Catalog, KernelError> {
    if catalog.kind == CatalogKind::Commands {
        let mut entries = Vec::new();
        for entry in catalog.entries {
            let Ok(spec) = serde_json::from_value::<CommandSpec>(entry.meta.clone()) else {
                entries.push(entry);
                continue;
            };
            let answer = json::dispatch(
                catalogue::COMMAND_DESCRIBE,
                links,
                cx,
                spec,
                Next::new(|spec| Box::pin(async { Ok(Some(spec)) })),
                report.clone(),
            )
            .await?;
            if let Some(spec) = answer {
                entries.push(CatalogEntry {
                    id: spec.name.clone(),
                    label: spec.hint.clone(),
                    meta: serde_json::to_value(spec).unwrap_or_default(),
                });
            }
        }
        catalog.entries = entries;
    }
    json::dispatch(
        catalogue::PROMPT_AUTOCOMPLETE,
        links,
        cx,
        catalog,
        Next::new(|catalog| Box::pin(async { Ok(catalog) })),
        report,
    )
    .await
}
