use bingo_sdk::{ErrorCode, KernelError};

pub(crate) fn invalid(message: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::InvalidInput, message.to_string())
}

pub(crate) fn internal(message: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::Internal, message.to_string())
}
