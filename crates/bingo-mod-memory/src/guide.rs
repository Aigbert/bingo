use bingo_sdk::{Contribution, Page, Pages, Registrar};
use std::sync::Arc;
static PAGES: Pages = Pages(&[Page {
    name: "memory-panel",
    description: "Read and forget memories, instructions and experience.",
    body: include_str!("guide.md"),
}]);
pub fn contribution(registrar: &Registrar) -> Contribution {
    Contribution::service(&Pages::key(registrar.plugin_id()), Arc::new(PAGES), None)
}
