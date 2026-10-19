use bingo_sdk::CatalogEntry;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    Model(String),
}
impl Row {
    pub fn selectable(&self) -> bool {
        true
    }
    /// The model this row stands for, wherever it is listed.
    pub fn model(&self) -> Option<&str> {
        match self {
            Self::Model(id) => Some(id),
        }
    }
    /// Names the row across redraws, so a key acts on the row a person saw.
    pub fn key(&self) -> String {
        match self {
            Self::Model(id) => format!("model:{id}"),
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
pub fn rows(models: &[CatalogEntry], filter: &str, show_all: bool) -> Vec<Row> {
    models
        .iter()
        .filter(|m| (show_all || !hidden(m)) && matched(&m.id, filter))
        .map(|m| Row::Model(m.id.clone()))
        .collect()
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
pub(crate) fn hidden(model: &CatalogEntry) -> bool {
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
    #[test]
    fn catalogue_order_and_identity_survive_insertion() {
        let (mut models, _) = crate::test_support::entries();
        let selected = format!("model:{}", models[1].id);
        models.insert(
            0,
            CatalogEntry {
                id: "p/new".into(),
                label: "new".into(),
                meta: serde_json::json!({}),
            },
        );
        let rows = rows(&models, "", false);
        assert_eq!(rows[0], Row::Model("p/new".into()));
        assert_eq!(
            rows[cursor(&rows, Some(&selected)).unwrap()].key(),
            selected
        );
        assert_eq!(
            rows.iter()
                .filter(|r| r.model() == Some("p/second"))
                .count(),
            1
        );
    }
}
