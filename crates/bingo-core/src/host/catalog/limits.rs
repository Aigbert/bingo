//! Publish resolved limits through the catalogue so clients share the switch ruler.
use crate::context::budget::Thresholds;
use crate::host::Host;
use crate::models::Resolved;
use bingo_sdk::{CatalogEntry, Provider};
use serde_json::json;
use std::sync::Arc;

pub(in crate::host) fn apply(
    host: &Host,
    providers: &[Arc<dyn Provider>],
    entries: &mut [CatalogEntry],
) {
    for entry in entries {
        let Some((id, model)) = entry.id.split_once('/') else {
            continue;
        };
        let Some(provider) = providers.iter().find(|p| p.id() == id) else {
            continue;
        };
        let declared = host.settings.kernel.models.get(&entry.id);
        let learned = host.learned.window(id, model);
        let endpoint = provider.endpoint(model);
        let Resolved {
            capabilities: caps,
            window_known,
        } = host.resolve_model(provider.as_ref(), model);
        // A guessed window draws no switch line: a person would act on it.
        if window_known {
            let max_tokens = crate::models::max_tokens(&caps, host.settings.kernel.max_tokens);
            entry.meta["switchAt"] = json!(Thresholds::of(caps.context_window, max_tokens).trigger);
            entry.meta["context"] = json!(caps.context_window);
        }
        if entry.meta.get("output").is_some() || declared.is_some_and(|d| d.max_output.is_some()) {
            entry.meta["output"] = json!(caps.max_output);
        }
        if entry.meta.get("images").is_some() || declared.is_some_and(|d| d.images.is_some()) {
            entry.meta["images"] = json!(caps.images);
        }
        if entry.meta.get("reasoning").is_some() || declared.is_some_and(|d| d.reasoning.is_some())
        {
            entry.meta["reasoning"] = json!(caps.reasoning);
            entry.meta["efforts"] = json!(if caps.reasoning {
                provider.efforts(model)
            } else {
                vec![]
            });
        }
        if declared.is_some() {
            entry.meta["limitsSource"] = json!("configured");
        } else if endpoint.context_window.is_some() || learned.is_some() {
            entry.meta["limitsSource"] = json!("endpoint");
        }
    }
}
