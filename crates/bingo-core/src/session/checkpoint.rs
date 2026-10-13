//! A checkpoint is acknowledged before the actor releases its covered window.

use super::{Actor, Msg};
use bingo_sdk::*;

impl Actor {
    pub(super) fn write_checkpoint(&self) {
        let Some(store) = self.store.clone() else {
            return;
        };
        let mut state = self.state.clone();
        state.signals.clear();
        state.transcript.drop_before_cut();
        let checkpoint = Checkpoint::new(state);
        let mailbox = self.mailbox.clone();
        let session = self.id.clone();
        self.tracker.spawn(async move {
            let result = store.checkpoint(&session, &checkpoint).await;
            mailbox.send(Msg::Checkpointed {
                seq: checkpoint.seq,
                result,
            });
        });
    }

    pub(super) fn checkpointed(&mut self, seq: Seq, result: Result<(), KernelError>) {
        if let Err(error) = result {
            tracing::warn!(session = %self.id, %error, "checkpoint write failed; keeping window");
        } else if seq > self.checkpoint_seq {
            self.checkpoint_seq = seq;
            self.journal.retain(|frame| frame.seq > seq);
            self.state.transcript.drop_before_cut();
        }
    }
}
