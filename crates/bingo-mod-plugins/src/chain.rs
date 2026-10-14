use bingo_sdk::{CatalogEntry, LinkMatcher, catalogue};
use serde_json::json;
/// The dispatch order, not catalogue or alphabetical order.
pub fn chain_of(site: &str, plugins: &[CatalogEntry]) -> Vec<String> {
    let mut links = plugins
        .iter()
        .flat_map(|plugin| {
            plugin.meta["links"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(move |link| {
                    let matcher: LinkMatcher =
                        serde_json::from_value(link["matcher"].clone()).ok()?;
                    renders(&matcher, site).then(|| {
                        (
                            link["order"].as_u64().unwrap_or(u64::MAX),
                            plugin.id.clone(),
                        )
                    })
                })
        })
        .collect::<Vec<_>>();
    links.sort_by_key(|(order, _)| *order);
    links
        .into_iter()
        .map(|(_, name)| name)
        .chain(std::iter::once("plain".into()))
        .collect()
}
pub fn sites(plugin: &CatalogEntry) -> Vec<String> {
    bingo_sdk::render::Component::ALL
        .into_iter()
        .filter(|site| !site.sealed())
        .filter_map(|site| {
            let name = serde_json::to_value(site).ok()?.as_str()?.to_string();
            (chain_of(&name, std::slice::from_ref(plugin)).len() > 1).then_some(name)
        })
        .collect()
}
pub fn drawn_over(plugin: &CatalogEntry, plugins: &[CatalogEntry]) -> Option<String> {
    sites(plugin).iter().find_map(|site| {
        chain_of(site, plugins)
            .first()
            .filter(|owner| **owner != plugin.id)
            .cloned()
    })
}

pub fn hooks(plugin: &CatalogEntry) -> Vec<String> {
    let mut hooks = Vec::new();
    for link in plugin.meta["links"].as_array().into_iter().flatten() {
        if let Ok(matcher) = serde_json::from_value::<LinkMatcher>(link["matcher"].clone()) {
            for pattern in matcher.patterns {
                let text = serde_json::to_value(pattern)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_string));
                if let Some(text) = text.filter(|text| text != "ui.render")
                    && !hooks.contains(&text)
                {
                    hooks.push(text);
                }
            }
        }
    }
    hooks
}

fn renders(matcher: &LinkMatcher, site: &str) -> bool {
    let input = json!({"component":site});
    matcher.matches(catalogue::UI_RENDER, &input)
        && matcher.render.as_ref().is_none_or(|partials| {
            partials
                .iter()
                .any(|partial| partial.is_null() || catalogue::prefilter(partial, &input))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chains_follow_dispatch_order_and_end_in_plain() {
        let make = |id: &str, order| CatalogEntry {
            id: id.into(),
            label: id.into(),
            meta: json!({"links":[{"order":order,"matcher":LinkMatcher { prefilter:json!({"component":"Palette"}), ..LinkMatcher::at(catalogue::UI_RENDER)}}]}),
        };
        let rows = [make("builtin", 5), make("outside", 1)];
        assert_eq!(chain_of("Palette", &rows), ["outside", "builtin", "plain"]);
        assert_eq!(chain_of("Plugins", &rows), ["plain"]);
        let mut grouped = make("grouped", 2);
        grouped.meta["links"][0]["matcher"] = json!(LinkMatcher {
            render: Some(vec![json!({"component":"Plugins"})]),
            ..LinkMatcher::at(catalogue::UI_RENDER)
        });
        assert_eq!(chain_of("Palette", &[grouped.clone()]), ["plain"]);
        assert_eq!(chain_of("Plugins", &[grouped]), ["grouped", "plain"]);
    }
}
