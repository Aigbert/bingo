//! The gate's verdict selects only a one-call allow; policy owns wider scope.
use agent_client_protocol_schema::v1::{
    PermissionOptionId, PermissionOptionKind, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SelectedPermissionOutcome,
};
use bingo_sdk::{CallLocation, ExternalCall, Verdict};
use serde_json::Value;

pub fn narrowest(
    request: &RequestPermissionRequest,
    verdict: &Verdict,
) -> Option<PermissionOptionId> {
    if matches!(verdict, Verdict::Allow { .. }) {
        if let Some(option) = offered(request, PermissionOptionKind::AllowOnce) {
            return Some(option);
        }
    }
    refusing(request)
}
pub fn refusing(request: &RequestPermissionRequest) -> Option<PermissionOptionId> {
    offered(request, PermissionOptionKind::RejectOnce)
        .or_else(|| offered(request, PermissionOptionKind::RejectAlways))
}
fn offered(
    request: &RequestPermissionRequest,
    kind: PermissionOptionKind,
) -> Option<PermissionOptionId> {
    request
        .options
        .iter()
        .find(|o| o.kind == kind)
        .map(|o| o.option_id.clone())
}
pub fn selected(option: PermissionOptionId) -> RequestPermissionResponse {
    RequestPermissionResponse::new(RequestPermissionOutcome::Selected(
        SelectedPermissionOutcome::new(option),
    ))
}
pub fn call(request: &RequestPermissionRequest) -> ExternalCall {
    let fields = &request.tool_call.fields;
    ExternalCall {
        id: request.tool_call.tool_call_id.to_string(),
        kind: fields
            .kind
            .map(crate::events::kind)
            .unwrap_or(bingo_sdk::CallKind::Other),
        title: fields
            .title
            .clone()
            .unwrap_or_else(|| request.tool_call.tool_call_id.to_string()),
        locations: fields
            .locations
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|l| CallLocation {
                path: l.path.clone(),
                line: l.line,
            })
            .collect(),
        raw: fields.raw_input.clone().unwrap_or(Value::Null),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn session_allow_still_selects_once_and_missing_allow_rejects() {
        let mut request: RequestPermissionRequest =
            serde_json::from_value(crate::fixtures::request_permission()).unwrap();
        let allow = Verdict::Allow {
            scope: Some("session".into()),
        };
        let picked = narrowest(&request, &allow).unwrap();
        assert_eq!(
            request
                .options
                .iter()
                .find(|o| o.option_id == picked)
                .unwrap()
                .kind,
            PermissionOptionKind::AllowOnce
        );
        request
            .options
            .retain(|o| o.kind != PermissionOptionKind::AllowOnce);
        assert_eq!(narrowest(&request, &allow), refusing(&request));
        request.options.retain(|o| {
            !matches!(
                o.kind,
                PermissionOptionKind::RejectOnce | PermissionOptionKind::RejectAlways
            )
        });
        assert!(narrowest(&request, &allow).is_none());
    }
}
