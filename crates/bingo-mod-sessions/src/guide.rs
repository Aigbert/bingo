use bingo_sdk::{Contribution, Page, Pages, Registrar};
use std::sync::Arc;
static PAGES: Pages = Pages(&[Page {
    name: "sessions",
    description: "Search commands, keys and sessions in one sessions.",
    body: include_str!("guide.md"),
}]);
pub fn contribution(registrar: &Registrar) -> Contribution {
    Contribution::service(&Pages::key(registrar.plugin_id()), Arc::new(PAGES), None)
}
