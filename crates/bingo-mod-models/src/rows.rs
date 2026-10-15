use bingo_sdk::CatalogEntry;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    Heading,
    /// A pinned model at the top; the same model is listed again under its provider.
    Pinned(String),
    Model(String),
    Provider(String),
    Hidden(String, usize),
    Add,
}
impl Row {
    pub fn selectable(&self) -> bool {
        !matches!(self, Self::Heading)
    }
    /// The model this row stands for, wherever it is listed.
    pub fn model(&self) -> Option<&str> {
        match self {
            Self::Pinned(id) | Self::Model(id) => Some(id),
            _ => None,
        }
    }
    /// Names the row across redraws, so a key acts on the row a person saw.
    pub fn key(&self) -> String {
        match self {
            Self::Heading => "heading".into(),
            Self::Pinned(id) => format!("pin:{id}"),
            Self::Model(id) => format!("model:{id}"),
            Self::Provider(id) => format!("provider:{id}"),
            Self::Hidden(id, _) => format!("hidden:{id}"),
            Self::Add => "add".into(),
        }
    }
}
/// The row a key names, if it is still listed and can be chosen.
pub fn named(rows: &[Row], key: &str) -> Option<usize> {
    rows.iter().position(|r| r.selectable() && r.key() == key)
}
/// Where the cursor stands: on the kept row while it is listed, else on the
/// first model, else on the first row that can be chosen; never on a heading.
pub fn cursor(rows: &[Row], selected: Option<&str>) -> Option<usize> {
    selected
        .and_then(|key| named(rows, key))
        .or_else(|| rows.iter().position(|r| r.model().is_some()))
        .or_else(|| rows.iter().position(Row::selectable))
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
            .map(|m| Row::Pinned(m.id.clone()))
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
/// A filter that lists no model offers itself as the model to use: a
/// `provider/model` as typed, a bare name under the current model's provider.
/// The id always names its provider, so `/model` takes it as a switch.
pub fn unlisted(rows: &[Row], filter: &str, current: &str) -> Option<String> {
    if filter.is_empty() || rows.iter().any(|r| r.model().is_some()) {
        return None;
    }
    let (provider, model) = match filter.split_once('/') {
        Some(pair) => pair,
        None => (current.split_once('/')?.0, filter),
    };
    (!provider.is_empty() && !model.is_empty()).then(|| format!("{provider}/{model}"))
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
                Row::Pinned("p/a".into()),
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
                .filter(|r| r.model().is_some())
                .count(),
            4
        );
        assert!(rows(&models, &providers, "other/", false).is_empty());
        let listed = rows(&models, &providers, "", false);
        assert_eq!(cursor(&listed, Some("model:p/a")), Some(3));
        assert_eq!(cursor(&listed, Some("model:p/gone")), Some(1));
        assert_eq!(cursor(&listed, Some("heading")), Some(1));
        assert_eq!(cursor(&listed, None), Some(1));
        assert_eq!(cursor(&[Row::Heading, Row::Add], None), Some(1));
        assert_eq!(cursor(&[], Some("add")), None);
        let unlisted_as = |filter: &str, current: &str| {
            unlisted(&rows(&models, &providers, filter, false), filter, current)
        };
        assert_eq!(unlisted_as("p/zzz", "q/x").as_deref(), Some("p/zzz"));
        assert_eq!(unlisted_as("zzz", "q/x").as_deref(), Some("q/zzz"));
        assert_eq!(unlisted_as("zzz", "/"), None, "no provider to name");
        assert_eq!(unlisted_as("other/", "q/x"), None, "no model to name");
        assert_eq!(unlisted_as("p/a", "q/x"), None);
        assert_eq!(unlisted_as("", "q/x"), None);
    }
}
