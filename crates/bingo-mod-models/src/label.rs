use bingo_sdk::CatalogEntry;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reach {
    Ready,
    Missing,
    Unchecked,
    Down,
}
pub fn reach(provider: &CatalogEntry) -> Reach {
    match provider.meta["auth"]["kind"].as_str() {
        Some("missing" | "expired") => Reach::Missing,
        _ => match provider.meta["reach"]["kind"].as_str() {
            Some("ok") => Reach::Ready,
            Some("down") => Reach::Down,
            _ => Reach::Unchecked,
        },
    }
}
pub fn label(model: &CatalogEntry, provider: &CatalogEntry, used: u64, keep: bool) -> String {
    if model.meta["chat"] == false {
        return "can't chat".into();
    }
    match reach(provider) {
        Reach::Missing => "sign in, then use".into(),
        Reach::Down => format!("fix {}", provider.id),
        Reach::Unchecked => "check, then use".into(),
        Reach::Ready if compacts(model, used) => "compact, then use".into(),
        Reach::Ready => if keep { "use and keep" } else { "use here" }.into(),
    }
}
pub fn compacts(model: &CatalogEntry, used: u64) -> bool {
    model.meta["switchAt"]
        .as_u64()
        .is_some_and(|threshold| used >= threshold)
}
pub fn status(provider: &CatalogEntry) -> String {
    match reach(provider) {
        Reach::Missing => "○ not signed in".into(),
        Reach::Unchecked => "◌ not checked".into(),
        Reach::Down => format!(
            "✗ {}",
            provider.meta["reach"]["error"]
                .as_str()
                .unwrap_or("not answering")
        ),
        Reach::Ready => {
            let source = provider.meta["auth"]["from"]
                .as_str()
                .unwrap_or("no key needed");
            let ms = provider.meta["reach"]["ms"].as_u64().unwrap_or_default();
            format!("● {source}  {ms} ms")
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn action_priority_is_auth_reach_window_then_scope() {
        let model = CatalogEntry {
            id: "p/m".into(),
            label: "m".into(),
            meta: json!({"context":100,"switchAt":90}),
        };
        for (auth, reach, used, keep, want) in [
            ("missing", "down", 99, true, "sign in, then use"),
            ("ready", "down", 99, true, "fix p"),
            ("ready", "unchecked", 99, true, "check, then use"),
            ("ready", "ok", 91, true, "compact, then use"),
            ("ready", "ok", 89, true, "use and keep"),
            ("ready", "ok", 1, false, "use here"),
        ] {
            let provider = CatalogEntry {
                id: "p".into(),
                label: "p".into(),
                meta: json!({"auth":{"kind":auth},"reach":{"kind":reach}}),
            };
            assert_eq!(label(&model, &provider, used, keep), want);
        }
    }
}
