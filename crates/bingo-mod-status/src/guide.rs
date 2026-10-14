use bingo_sdk::{Contribution, Page, Pages, Registrar};
use std::sync::Arc;
static PAGES: Pages = Pages(&[Page {
    name: "status",
    description: "Inspect the live session and open the command that changes each row.",
    body: include_str!("guide.md"),
}]);
pub fn contribution(registrar: &Registrar) -> Contribution {
    Contribution::service(&Pages::key(registrar.plugin_id()), Arc::new(PAGES), None)
}
