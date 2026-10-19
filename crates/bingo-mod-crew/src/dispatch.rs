use bingo_sdk::*;
use serde_json::json;
pub async fn model(cx: &CommandContext, id: &SessionId) -> Result<CommandOutcome, KernelError> {
    let mut attached = crate::live::attach(&cx.host, id).await?;
    let intent = IntentId::mint();
    let mut origin = Origin::surface(format!("plugin:{}", crate::ID));
    origin.render_surface = cx.surface;
    attached.handle.submit(
        intent.clone(),
        Input::action(
            Action {
                name: "model".into(),
                args: json!(""),
            },
            origin,
        ),
    );
    loop {
        let frame = tokio::select! {
            _ = cx.cancel.cancelled() => return Err(crate::error("model picker cancelled")),
            frame = std::future::poll_fn(|cx| attached.events.as_mut().poll_next(cx)) => frame,
        };
        let Some(frame) = frame else {
            return Err(crate::error("model session closed"));
        };
        if let Event::IntentAck {
            intent: ack,
            outcome,
        } = frame.event
            && ack == intent
        {
            return result(outcome, id);
        }
    }
}
fn result(outcome: IntentOutcome, id: &SessionId) -> Result<CommandOutcome, KernelError> {
    match outcome {
        IntentOutcome::Rejected { error } => Err(error),
        IntentOutcome::Queued { position } => Ok(CommandOutcome::Applied {
            message: Some(format!("queued (position {position})")),
        }),
        IntentOutcome::TurnStarted { .. } => Ok(CommandOutcome::Handled { handled: true }),
        IntentOutcome::Applied { mut result } => {
            if result["surface"].is_object() {
                result["surface"]["args"]["view"] = json!(id);
            }
            Ok(CommandOutcome::Acknowledged { result })
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queued_and_started_acknowledgements_finish_dispatch() {
        let id = SessionId::from_raw("ses_child");
        assert!(
            matches!(result(IntentOutcome::Queued { position: 2 }, &id).unwrap(),
            CommandOutcome::Applied { message: Some(message) } if message == "queued (position 2)")
        );
        assert_eq!(
            result(
                IntentOutcome::TurnStarted {
                    turn: TurnId::from_raw("trn_1")
                },
                &id
            )
            .unwrap(),
            CommandOutcome::Handled { handled: true }
        );
    }
}
