//! Close the unfinished segment of a recovered journal.

use super::*;

/// A resumed journal may end inside a turn the old process never
/// finished. Its questions can no longer be answered and its turn is
/// lost; both are said before anything new happens.
pub(super) async fn recover(actor: &mut Actor) -> Result<(), KernelError> {
    let open: Vec<InteractionId> = actor
        .state
        .interactions
        .iter()
        .map(|i| i.id.clone())
        .collect();
    for id in open {
        actor
            .publish(
                Event::InteractionCancelled {
                    id,
                    reason: CancelReason::SessionClosed,
                },
                None,
            )
            .await?;
    }
    if let Some(turn) = actor.state.turn.clone() {
        recover_items(actor, &turn.id).await?;
        actor
            .publish(
                Event::TurnCompleted {
                    turn: turn.id,
                    status: TurnStatus::Failed {
                        error: KernelError::new(
                            ErrorCode::TurnLost,
                            "the process ended during this turn",
                        ),
                    },
                    usage: turn.usage,
                },
                None,
            )
            .await?;
    }
    Ok(())
}

async fn recover_items(actor: &mut Actor, turn: &TurnId) -> Result<(), KernelError> {
    let open: Vec<_> = actor.state.transcript.open_items(turn).cloned().collect();
    for mut item in open {
        item.status = ItemStatus::Interrupted;
        item.completed_at = Some(Timestamp::now());
        actor.record_item(item).await?;
    }
    Ok(())
}
