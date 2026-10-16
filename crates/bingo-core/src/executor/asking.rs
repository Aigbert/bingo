use bingo_sdk::*;
use std::sync::Arc;

pub(super) fn bind(mut cx: ToolContext) -> ToolContext {
    cx.call = Arc::new(Asker {
        item: cx.item.clone(),
        host: cx.call,
    });
    cx
}

struct Asker {
    item: ItemId,
    host: Arc<dyn ToolHost>,
}

#[async_trait::async_trait]
impl Prompter for Asker {
    async fn ask(
        &self,
        kind: InteractionKind,
        answers: Vec<AnswerSpec>,
    ) -> Result<Answer, KernelError> {
        self.host.ask_for(&self.item, kind, answers).await
    }
}

#[async_trait::async_trait]
impl ToolHost for Asker {
    fn progress(&self, item: &ItemId, tail: String) {
        self.host.progress(item, tail);
    }
    async fn record(&self, body: ItemBody) -> Result<ItemId, KernelError> {
        self.host.record(body).await
    }
}
