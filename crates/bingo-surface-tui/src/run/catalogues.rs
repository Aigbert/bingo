use super::*;

/// The catalogues whose ids a command's argument may name, by the source name
/// its `ArgSpec::Catalog` gives.
const CATALOGUES: &[(&str, CatalogKind)] = &[
    ("models", CatalogKind::Models),
    ("providers", CatalogKind::Providers),
    ("plugins", CatalogKind::Plugins),
];

/// The catalogues are read at start and again whenever the host says one
/// of them was rebuilt: the dropdown follows, it does not poll.
pub(super) fn fetch_all(run: &mut Run) {
    fetch_commands(run);
    for (source, kind) in CATALOGUES {
        fetch(run, source, *kind);
    }
}

pub(super) fn fetch_commands(run: &mut Run) {
    let host = run.host.clone();
    run.spawn(async move {
        host.catalog(CatalogKind::Commands)
            .await
            .map(|c| Reply::Commands(commands::specs_from(&c)))
    });
}

/// One catalogue's ids, on the loop's own host-call lane.
pub(super) fn fetch(run: &mut Run, source: &'static str, kind: CatalogKind) {
    let host = run.host.clone();
    run.spawn(async move {
        host.catalog(kind)
            .await
            .map(|c| Reply::Catalogue(source.to_string(), c.entries))
    });
}

/// A catalogue the host rebuilt — a provider's endpoint answered what it
/// serves (ADR-0026 §4) — is read again, and only that one.
pub(super) fn changed(run: &mut Run, kind: CatalogKind) {
    if kind == CatalogKind::Plugins && crate::plugins::active(&run.ui) {
        run.ui.placed.pulled.invalidate();
    }
    if kind == CatalogKind::Commands {
        run.ui.placed.pulled.invalidate();
        fetch_commands(run);
        return;
    }
    let Some((source, _)) = CATALOGUES.iter().find(|(_, listed)| *listed == kind) else {
        return;
    };
    fetch(run, source, kind);
}
