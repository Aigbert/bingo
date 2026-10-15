//! Publish resolved limits through the catalogue so clients share the switch ruler.
use super::*;
use crate::context::budget::Thresholds;
use serde_json::json;

pub(super) fn apply(host: &Host, providers: &[Arc<dyn Provider>], entries: &mut [CatalogEntry]) {
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
        let caps = host.resolve_model(provider.as_ref(), model);
        let max_tokens = crate::models::max_tokens(&caps, host.settings.kernel.max_tokens);
        entry.meta["switchAt"] = json!(Thresholds::of(caps.context_window, max_tokens).trigger);
        if entry.meta.get("context").is_some()
            || declared.is_some_and(|d| d.context_window.is_some())
            || learned.is_some()
            || endpoint.context_window.is_some()
        {
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
