use bingo_sdk::*;
use std::sync::{Arc, Mutex};

struct Recording {
    inner: SessionHandle,
    inputs: Arc<Mutex<Vec<Input>>>,
}
#[async_trait::async_trait]
impl SessionPort for Recording {
    fn submit(&self, _: IntentId, input: Input) {
        self.inputs.lock().unwrap().push(input);
    }
    fn interrupt(&self, id: IntentId, scope: InterruptScope) {
        self.inner.interrupt(id, scope);
    }
    fn answer(
        &self,
        id: IntentId,
        interaction: InteractionId,
        answer: Answer,
        activation: Activation,
    ) {
        self.inner.answer(id, interaction, answer, activation);
    }
    async fn history(&self, page: HistoryPage) -> Result<HistoryChunk, KernelError> {
        self.inner.history(page).await
    }
    async fn events_since(&self, since: Seq) -> Result<FrameStream, KernelError> {
        self.inner.events_since(since).await
    }
}
#[tokio::test]
async fn the_child_receives_a_signed_action_with_render_metadata() {
    let row = crate::cards_tests::row("reviewer", "idle");
    let id = row.summary.id.clone();
    let table = testing::TestHost::with_sessions(vec![row.summary]).handle();
    let inputs = Arc::new(Mutex::new(Vec::new()));
    let captured = inputs.clone();
    let host = testing::TestHost::default();
    host.on_open(move |selector, who, options| {
        assert_eq!(who.name, crate::ID);
        assert_eq!(who.surface, "terminal");
        let (table, inputs) = (table.clone(), captured.clone());
        async move {
            let mut attachment = table.open(selector, who, options).await?;
            attachment.handle = SessionHandle(Arc::new(Recording {
                inner: attachment.handle,
                inputs,
            }));
            Ok(attachment)
        }
    });
    let cx = CommandContext {
        surface: None,
        session: SessionId::from_raw("ses_parent"),
        cwd: "/unused".into(),
        host: host.handle(),
        cancel: Default::default(),
    };
    assert!(
        crate::dispatch::model(&cx, &id).await.is_err(),
        "the recording stream ends without an ack"
    );
    let inputs = inputs.lock().unwrap();
    let [
        Input::Action {
            action,
            origin: Some(origin),
        },
    ] = inputs.as_slice()
    else {
        panic!("expected one signed action: {inputs:?}");
    };
    assert_eq!(action.name, "model");
    assert_eq!(action.args, serde_json::json!(""));
    assert_eq!(origin.surface, "plugin:bingo.mod.crew");
    assert_eq!(origin.render_surface, Some(render::SurfaceName::Terminal));
}
