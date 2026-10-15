//! What a fire submits and how it hears back (ADR-0094 §2).
//!
//! A person who asked for an entry with their own `/schedule add` or
//! `/schedule confirm` gave consent then, for exactly that line and mode;
//! the fire carries it out later with nobody at the keyboard. That is what a
//! `--print` line is, so the person's entry is signed as one. Everything else
//! a schedule says keeps the schedule's own origin.

use bingo_sdk::{
    Attachment, CancellationToken, ErrorCode, Event, Input, IntentId, IntentOutcome, KernelError,
    Origin, Voice,
};

/// The surface a person's line runs on when nobody is at the keyboard.
const UNATTENDED: &str = "print";

/// Who the person's own entry speaks as when it fires.
pub fn asked_origin() -> Origin {
    Origin {
        render_surface: None,
        voice: Some(Voice::Said),
        surface: UNATTENDED.into(),
        principal: None,
        conversation: None,
    }
}

/// Submit `input` and wait for the session's answer to it: refused, or
/// taken (applied, queued or a turn started).
pub async fn submitted(
    attachment: &mut Attachment,
    input: Input,
    cancel: &CancellationToken,
) -> Result<(), KernelError> {
    let intent = IntentId::mint();
    attachment.handle.submit(intent.clone(), input);
    acknowledged(attachment, &intent, cancel).await
}

pub(crate) async fn acknowledged(
    attachment: &mut Attachment,
    intent: &IntentId,
    cancel: &CancellationToken,
) -> Result<(), KernelError> {
    loop {
        let frame = tokio::select! {
            _ = cancel.cancelled() => {
                return Err(KernelError::new(ErrorCode::Interrupted, "the scheduler stopped"));
            }
            frame = std::future::poll_fn(|cx| attachment.events.as_mut().poll_next(cx)) => frame,
        };
        let Some(frame) = frame else {
            return Err(KernelError::new(
                ErrorCode::SessionClosed,
                "the schedule's session closed",
            ));
        };
        if let Event::IntentAck {
            intent: answered,
            outcome,
        } = frame.event
            && &answered == intent
        {
            return taken(outcome);
        }
    }
}

fn taken(outcome: IntentOutcome) -> Result<(), KernelError> {
    match outcome {
        IntentOutcome::Rejected { error } => Err(error),
        IntentOutcome::Applied { .. }
        | IntentOutcome::Queued { .. }
        | IntentOutcome::TurnStarted { .. } => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_persons_entry_speaks_as_an_unattended_person_line() {
        let origin = asked_origin();
        assert!(bingo_sdk::PERSON_SURFACES.contains(&origin.surface.as_str()));
        assert_eq!(bingo_sdk::effective_voice(&origin), Voice::Said);
        assert_eq!(
            origin.principal, None,
            "no correspondent stands in for them"
        );
    }

    #[test]
    fn only_a_refusal_is_an_error() {
        let refused = KernelError::new(ErrorCode::PermissionDenied, "no");
        assert_eq!(
            taken(IntentOutcome::Rejected {
                error: refused.clone()
            }),
            Err(refused)
        );
        assert_eq!(taken(IntentOutcome::Queued { position: 1 }), Ok(()));
        assert_eq!(
            taken(IntentOutcome::Applied {
                result: serde_json::Value::Null
            }),
            Ok(())
        );
    }
}
