use async_trait::async_trait;
use bingo_sdk::{Answer, AnswerSpec, InteractionId, InteractionKind, KernelError};
use jiff::Timestamp;

#[derive(Clone, Debug)]
pub struct Asked {
    pub id: InteractionId,
    pub opened_at: Timestamp,
}

#[derive(Debug)]
pub struct Answered {
    pub ask: Asked,
    pub answer: Answer,
}

/// The gate needs the identity of the ask that actually settled its call.
#[async_trait]
pub trait GatePrompter: Send + Sync {
    async fn ask(
        &self,
        kind: InteractionKind,
        answers: Vec<AnswerSpec>,
    ) -> Result<Answered, KernelError>;
}
