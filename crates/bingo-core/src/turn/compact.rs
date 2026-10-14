//! Manual compaction and its complete post-cut context measurement.

use super::*;

/// One manual compaction, then done. What it bought or did not is said
/// on the stream, as for any other compaction.
pub(super) async fn run(turn: &mut Turn<'_>, instructions: Option<String>) -> TurnStatus {
    let Some(compactor) = turn.late.compactor.clone() else {
        return TurnStatus::Failed {
            error: KernelError::new(
                ErrorCode::InvalidInput,
                "no compaction strategy is registered",
            ),
        };
    };
    let Some((request, usage)) = turn.assemble_request().await else {
        return turn.interrupted_status();
    };
    let compacted = turn
        .compact(
            compactor.as_ref(),
            CompactReason::Manual { instructions },
            usage,
            &request,
        )
        .await;
    if turn.cancel.is_cancelled() {
        return turn.interrupted_status();
    }
    if compacted {
        measure_compacted(turn, &request);
    }
    TurnStatus::Completed
}

/// Measure the retained transcript with the assembled system and tools.
/// Contributors stay lazy: measuring must not reissue context or refresh
/// a cached baseline before the next ordinary request needs it.
fn measure_compacted(turn: &mut Turn<'_>, request: &ModelRequest) {
    let context = turn.ruler.measure(estimate_tokens(
        &request.system,
        &ContextView::model_view(&turn.transcript),
        &request.tools,
    ));
    emit_and_fold(
        &mut turn.transcript,
        turn.host,
        Event::TurnUsage {
            turn: turn.id.clone(),
            usage: Usage::default(),
            context,
        },
    );
}
