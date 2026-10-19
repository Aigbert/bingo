use bingo_sdk::{Contribution, Page, Pages, Registrar};
use std::sync::Arc;
static PAGES: Pages = Pages(&[Page {
    name: "palette",
    description: "Search commands, skills and sessions in one palette.",
    body: include_str!("guide.md"),
}]);
pub fn contribution(registrar: &Registrar) -> Contribution {
    Contribution::service(&Pages::key(registrar.plugin_id()), Arc::new(PAGES), None)
}
