//! Surface protocol error vocabulary.
pub use bingo_jsonrpc::codec::*;
use bingo_sdk::{ErrorCode, KernelError};
use serde_json::json;
pub const KERNEL_ERROR: i64 = -32000;
/// The stable `ErrorCode` travels in `data.code`; the text stays for people.
pub fn from_kernel(error: KernelError) -> RpcError {
    RpcError {
        code: KERNEL_ERROR,
        message: error.message,
        data: Some(json!({ "code": error.code })),
    }
}

/// The inverse, so a `RemoteKernel` hands its caller the error the kernel raised.
pub fn to_kernel(error: RpcError) -> KernelError {
    let code = error
        .data
        .as_ref()
        .and_then(|data| data.get("code"))
        .and_then(|code| serde_json::from_value(code.clone()).ok())
        .unwrap_or_else(|| protocol_code(error.code));
    KernelError::new(code, error.message)
}

/// A protocol-level error carries no `data.code`; the client sent it, so it is input.
fn protocol_code(code: i64) -> ErrorCode {
    match code {
        INVALID_REQUEST | METHOD_NOT_FOUND | INVALID_PARAMS | PARSE_ERROR => {
            ErrorCode::InvalidInput
        }
        _ => ErrorCode::Internal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_kernel_error_keeps_its_code_across_the_wire() {
        let kernel = KernelError::new(ErrorCode::SessionNotFound, "no such session");
        let wire = from_kernel(kernel.clone());
        assert_eq!(wire.code, KERNEL_ERROR);
        assert_eq!(wire.data, Some(json!({ "code": "SESSION_NOT_FOUND" })));
        assert_eq!(to_kernel(wire), kernel);
    }

    #[test]
    fn a_protocol_error_comes_back_as_invalid_input() {
        let wire = RpcError::new(METHOD_NOT_FOUND, "no such method: nope");
        assert_eq!(to_kernel(wire).code, ErrorCode::InvalidInput);
    }
}
