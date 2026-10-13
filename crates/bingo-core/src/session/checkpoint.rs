//! A checkpoint is acknowledged before the actor releases its covered window.

use super::{Actor, Msg};
use bingo_sdk::*;

pub(super) fn write(actor: &Actor) {
    let Some(store) = actor.store.clone() else {
        return;
    };
    let mut state = actor.state.clone();
    state.signals.clear();
    state.transcript.drop_before_cut();
    let checkpoint = Checkpoint::new(state);
    let mailbox = actor.mailbox.clone();
    let session = actor.id.clone();
    actor.tracker.spawn(async move {
        let result = store.checkpoint(&session, &checkpoint).await;
        mailbox.send(Msg::Checkpointed {
            seq: checkpoint.seq,
            result,
        });
    });
}

pub(super) fn acknowledge(actor: &mut Actor, seq: Seq, result: Result<(), KernelError>) {
    if let Err(error) = result {
        tracing::warn!(session = %actor.id, %error, "checkpoint write failed; keeping window");
    } else if seq > actor.checkpoint_seq {
        actor.checkpoint_seq = seq;
        actor.journal.retain(|frame| frame.seq > seq);
        actor.state.transcript.drop_before_cut();
    }
}
