//! Person-input authorization and publication of a saved approval.
use super::*;

pub(super) fn approve(host: &Host, request: Approval) -> Result<String, KernelError> {
    if !bingo_sdk::render::asked() {
        return Err(KernelError::new(
            ErrorCode::PermissionDenied,
            "approval requires person input",
        ));
    }
    let digest = host.approvals.approve(&request)?;
    let _ = host.gateway.send(GatewayEvent::CatalogChanged {
        kind: CatalogKind::Plugins,
    });
    Ok(digest)
}
