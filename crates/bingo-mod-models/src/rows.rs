use bingo_sdk::CatalogEntry;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    Heading,
    Model(String),
    Provider(String),
    Hidden(String, usize),
    Add,
}
impl Row {
    pub fn selectable(&self) -> bool {
        !matches!(self, Self::Heading)
    }
}
pub fn rows(
    models: &[CatalogEntry],
    providers: &[CatalogEntry],
    filter: &str,
    show_all: bool,
) -> Vec<Row> {
    let mut rows = Vec::new();
    if filter.is_empty() {
        let pins: Vec<_> = models
            .iter()
            .filter(|m| m.meta["pinned"] == true)
            .map(|m| Row::Model(m.id.clone()))
            .collect();
        if !pins.is_empty() {
            rows.push(Row::Heading);
            rows.extend(pins);
        }
    }
    for provider in providers {
        let offered: Vec<_> = models
            .iter()
            .filter(|m| m.meta["provider"] == provider.id)
            .collect();
        let matches: Vec<_> = offered
            .iter()
            .filter(|m| (show_all || !hidden(m)) && matched(&m.id, filter))
            .collect();
        if !filter.is_empty() && matches.is_empty() && !matched(&provider.id, filter) {
            continue;
        }
        rows.push(Row::Provider(provider.id.clone()));
        rows.extend(matches.iter().map(|m| Row::Model(m.id.clone())));
        let hidden = offered.iter().filter(|m| hidden(m)).count();
        if !show_all && filter.is_empty() && hidden > 0 {
            rows.push(Row::Hidden(provider.id.clone(), hidden));
        }
    }
    if filter.is_empty() {
        rows.push(Row::Add);
    }
    rows
}
fn hidden(model: &CatalogEntry) -> bool {
    model.meta["chat"] == false || model.meta["aliasOf"].is_string()
}
fn matched(id: &str, query: &str) -> bool {
    let id = id.to_lowercase();
    let query = query.to_lowercase();
    if let Some((provider, model)) = query.split_once('/') {
        return id
            .split_once('/')
            .is_some_and(|(p, m)| p.starts_with(provider) && subsequence(m, model));
    }
    subsequence(&id, &query)
}
fn subsequence(text: &str, query: &str) -> bool {
    let mut letters = text.chars();
    query
        .chars()
        .all(|c| letters.by_ref().any(|letter| c == letter))
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn pins_groups_hidden_and_provider_prefixes() {
        let providers = vec![CatalogEntry {
            id: "p".into(),
            label: "p".into(),
            meta: json!({}),
        }];
        let models = [
            ("a", json!({"pinned":true})),
            ("a-20261015", json!({"aliasOf":"a"})),
            ("embed", json!({"chat":false})),
        ]
        .into_iter()
        .map(|(id, mut meta)| {
            meta["provider"] = json!("p");
            CatalogEntry {
                id: format!("p/{id}"),
                label: id.into(),
                meta,
            }
        })
        .collect::<Vec<_>>();
        assert_eq!(
            rows(&models, &providers, "", false),
            vec![
                Row::Heading,
                Row::Model("p/a".into()),
                Row::Provider("p".into()),
                Row::Model("p/a".into()),
                Row::Hidden("p".into(), 2),
                Row::Add
            ]
        );
        assert_eq!(
            rows(&models, &providers, "p/a", false),
            vec![Row::Provider("p".into()), Row::Model("p/a".into())]
        );
        assert_eq!(
            rows(&models, &providers, "", true)
                .iter()
                .filter(|r| matches!(r, Row::Model(_)))
                .count(),
            4
        );
        assert!(rows(&models, &providers, "other/", false).is_empty());
    }
}
