//! The live ordered pin setting and its one user-layer writer.

use super::*;

pub(crate) fn pinned_models(host: &Host) -> Vec<String> {
    host.pinned
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

pub(crate) fn pin_model(host: &Host, id: &str, pin: bool) -> Result<(), KernelError> {
    let mut pinned = host.pinned.lock().unwrap_or_else(|p| p.into_inner());
    let mut next = pinned.clone();
    if pin {
        if !next.iter().any(|entry| entry == id) {
            next.push(id.to_owned());
        }
    } else {
        next.retain(|entry| entry != id);
    }
    settings::remember(
        &settings::user_path(host.env()),
        &[("pinned", serde_json::json!(next))],
    )
    .map_err(|error| KernelError::new(ErrorCode::InvalidInput, error.to_string()))?;
    *pinned = next;
    let _ = host.gateway.send(GatewayEvent::CatalogChanged {
        kind: CatalogKind::Models,
    });
    Ok(())
}
