use bingo_sdk::*;
use serde_json::Value;

pub async fn run(command: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
    let mut attachment = crate::snapshot(&cx.host, &cx.session).await?;
    let intent = IntentId::mint();
    let (name, args) = command.split_once(' ').unwrap_or((command, ""));
    attachment.handle.submit(
        intent.clone(),
        // Signed as this mod's, never the person's: a row runs no verb
        // only a person may run.
        Input::action(
            Action {
                name: name.into(),
                args: Value::String(args.into()),
            },
            Origin::surface(format!("plugin:{}", crate::ID)),
        ),
    );
    // The instant site command holds no actor queue. Return the nested command's
    // result through the original key acknowledgement, which the surface owns.
    loop {
        let frame = tokio::select! {
            _ = cx.cancel.cancelled() => return Err(KernelError::new(ErrorCode::Interrupted, "status action interrupted")),
            frame = std::future::poll_fn(|cx| attachment.events.as_mut().poll_next(cx)) => frame,
        };
        let Some(frame) = frame else {
            return Err(KernelError::new(
                ErrorCode::SessionClosed,
                "status action session closed",
            ));
        };
        if let Event::IntentAck {
            intent: acknowledged,
            outcome,
        } = frame.event
            && acknowledged == intent
            && let Some(result) = result(outcome)
        {
            return result;
        }
    }
}

fn result(outcome: IntentOutcome) -> Option<Result<CommandOutcome, KernelError>> {
    Some(match outcome {
        IntentOutcome::Queued { position } => Ok(CommandOutcome::Applied {
            message: Some(format!("queued (position {position})")),
        }),
        IntentOutcome::Rejected { error } => Err(error),
        IntentOutcome::TurnStarted { .. } => Ok(CommandOutcome::Handled { handled: true }),
        IntentOutcome::Applied { result } => applied(result),
    })
}

fn applied(result: Value) -> Result<CommandOutcome, KernelError> {
    Ok(CommandOutcome::Acknowledged { result })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn acknowledgements_forward_views_messages_and_refusals() {
        assert_eq!(
            applied(json!({"view":View::text("next screen")})).unwrap(),
            CommandOutcome::Acknowledged {
                result: json!({"view":View::text("next screen")})
            }
        );
        assert_eq!(
            applied(json!({"message":"forgot rule"})).unwrap(),
            CommandOutcome::Acknowledged {
                result: json!({"message":"forgot rule"})
            }
        );
        assert_eq!(
            result(IntentOutcome::Queued { position: 1 }),
            Some(Ok(CommandOutcome::Applied {
                message: Some("queued (position 1)".into())
            }))
        );
        let error = KernelError::new(ErrorCode::InvalidInput, "refused");
        assert_eq!(
            result(IntentOutcome::Rejected {
                error: error.clone()
            }),
            Some(Err(error))
        );
    }
}

#[cfg(test)]
mod forwarding_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn acknowledgements_preserve_every_result_without_recording_twice() {
        for result in [
            json!({"view":View::text("screen")}),
            json!({"message":"done"}),
            json!({"surface":{"name":"Composer","args":{"text":"/rename "}},"message":"fallback"}),
            json!({"item":"existing-item"}),
            json!({"answer":{"kind":"cancel"},"handled":true}),
        ] {
            let forwarded = serde_json::to_value(applied(result.clone()).unwrap()).unwrap();
            assert_eq!(forwarded, json!({"kind":"acknowledged","result":result}));
        }
    }
}
