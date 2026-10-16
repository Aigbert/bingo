//! The events section: what each is, what `next` does, how to match it,
//! and the shapes it carries.
use super::{fields, one_line};
use bingo_sdk::{Catalog, CatalogEntry};
use serde_json::Value;

const HEAD: &str = "## Events\n\n`on(event, matcher?, hook)`; a hook gets `($, event, next)`. A chain hook may return the result shape to answer, call `next(event)` (or `next` with a changed copy to rewrite), or return `undefined` to keep what `next` gave. `acts` says which of deny, rewrite and answer the event allows; pinned fields cannot be rewritten. An observation's return is ignored.\n\n";

pub(super) fn section(catalog: &Catalog) -> String {
    let mut text = String::from(HEAD);
    for entry in &catalog.entries {
        text.push_str(&event(entry));
    }
    text
}

fn event(entry: &CatalogEntry) -> String {
    let meta = &entry.meta;
    let mut text = format!("### `{}`\n\n", entry.id);
    if let Some(doc) = crate::types::render::event_text(&entry.id, meta) {
        text.push_str(&format!("{}\n\n", one_line(&doc)));
    }
    text.push_str(&facts(meta));
    text.push_str("\nInput:\n");
    text.push_str(&fields(&meta["input"], &meta["input"]));
    text.push_str("\nResult:\n");
    text.push_str(&fields(&meta["result"], &meta["result"]));
    text.push('\n');
    text
}

fn facts(meta: &Value) -> String {
    let kind = meta["kind"].as_str().unwrap_or("chain");
    let mut line = format!("Kind: {kind}.");
    let acts: Vec<&str> = ["deny", "rewrite", "answer"]
        .into_iter()
        .filter(|act| meta["acts"][act] == true)
        .collect();
    if kind == "chain" {
        line.push_str(&format!(" Acts: {}.", listed(&acts, "none")));
    }
    line.push_str(&format!(
        " Match on: {}.",
        listed(&strings(&meta["matcher"]), "no fields")
    ));
    let pinned = strings(&meta["pinned"]);
    if !pinned.is_empty() {
        line.push_str(&format!(" Pinned: {}.", listed(&pinned, "")));
    }
    line.push('\n');
    line
}

fn strings(value: &Value) -> Vec<&str> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn listed(items: &[&str], none: &str) -> String {
    if items.is_empty() {
        return none.into();
    }
    items
        .iter()
        .map(|item| format!("`{item}`"))
        .collect::<Vec<_>>()
        .join(", ")
}
