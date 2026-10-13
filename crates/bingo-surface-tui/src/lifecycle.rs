//! A lifecycle word with the last turn's verdict and spend beside it.
use bingo_collab::lifecycle::Lifecycle;
use bingo_sdk::TurnStatus;

pub fn said(lifecycle: Lifecycle, _last: Option<&TurnStatus>, _spent: &str) -> String {
    lifecycle.word().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::{ErrorCode, InterruptReason, KernelError};

    #[test]
    fn the_verdict_never_replaces_idle() {
        for (status, word) in [
            (TurnStatus::Completed, "completed"),
            (
                TurnStatus::Interrupted {
                    reason: InterruptReason::UserCancel,
                },
                "interrupted",
            ),
            (
                TurnStatus::Failed {
                    error: KernelError::new(ErrorCode::Internal, "failed"),
                },
                "failed",
            ),
        ] {
            assert_eq!(
                said(
                    Lifecycle::Idle,
                    Some(&status),
                    "3 tools · 1.2k tokens · 40s"
                ),
                format!("idle · {word} · 3 tools · 1.2k tokens · 40s")
            );
        }
        assert_eq!(said(Lifecycle::Idle, None, ""), "idle");
        assert_eq!(
            said(Lifecycle::Working, Some(&TurnStatus::Completed), "3 tools"),
            "working · 3 tools"
        );
        assert_eq!(
            said(Lifecycle::Asking, Some(&TurnStatus::Completed), ""),
            "asking"
        );
        assert_eq!(
            said(Lifecycle::Stored, Some(&TurnStatus::Completed), ""),
            "stored"
        );
    }
}
