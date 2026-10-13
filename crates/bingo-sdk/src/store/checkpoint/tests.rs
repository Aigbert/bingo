use super::*;
use crate::*;

fn ids<'a>(items: impl Iterator<Item = &'a Item>) -> Vec<String> {
    items.map(|item| item.id.to_string()).collect()
}

#[test]
fn checkpoint_fixture_preserves_model_and_client_orders() {
    let bytes = include_str!("../../../fixtures/checkpoint-v1.json");
    let checkpoint: Checkpoint = serde_json::from_str(bytes).unwrap();
    assert_eq!(
        ids(checkpoint
            .state
            .transcript
            .items()
            .iter()
            .map(AsRef::as_ref)),
        ["i3", "i4"]
    );
    assert_eq!(
        ids(checkpoint.state.transcript.model_items()),
        ["i1", "i4", "i3"]
    );
    assert!(checkpoint.state.transcript.history().has_more);
    let restored: Checkpoint =
        serde_json::from_slice(&serde_json::to_vec(&checkpoint).unwrap()).unwrap();
    assert_eq!(restored, checkpoint);
    assert_eq!(
        serde_json::to_value(&checkpoint).unwrap(),
        serde_json::from_str::<serde_json::Value>(bytes).unwrap()
    );
}

struct ReplayOnly(Vec<Frame>);

#[async_trait::async_trait]
impl SessionStore for ReplayOnly {
    async fn create(&self, _: &SessionSummary) -> Result<(), KernelError> {
        Ok(())
    }
    async fn append(&self, _: &SessionId, _: &Frame) -> Result<(), KernelError> {
        Ok(())
    }
    async fn replay(&self, _: &SessionId, since: Seq) -> Result<Vec<Frame>, KernelError> {
        assert_eq!(since, Seq::ZERO);
        Ok(self.0.clone())
    }
    async fn list(&self, _: &SessionFilter) -> Result<Vec<SessionSummary>, KernelError> {
        Ok(vec![])
    }
    async fn delete(&self, _: &SessionId) -> Result<(), KernelError> {
        Ok(())
    }
}

#[tokio::test]
async fn checkpoint_defaults_replay_without_losing_frames() {
    let event: Event = serde_json::from_str(include_str!(
        "../../../fixtures/record/event/sessionUpdated.json"
    ))
    .unwrap();
    let Event::SessionUpdated { summary } = &event else {
        panic!("summary")
    };
    let state = SessionState::new(summary.clone());
    let id = summary.id.clone();
    let frame = Frame {
        seq: Seq(1),
        ts: jiff::Timestamp::UNIX_EPOCH,
        session: id.clone(),
        cause: None,
        event,
    };
    let store = ReplayOnly(vec![frame]);
    store
        .checkpoint(&id, &Checkpoint::new(state))
        .await
        .unwrap();
    assert_eq!(
        store.resume(&id).await.unwrap(),
        Resumed {
            state: None,
            tail: store.0
        }
    );
}

#[test]
fn checkpoint_codec_preserves_opaque_item_bytes() {
    let mut checkpoint: Checkpoint =
        serde_json::from_str(include_str!("../../../fixtures/checkpoint-v1.json")).unwrap();
    let raw = r#"{ "kind" : "future", "payload" : [1,  2] }"#;
    let body: ItemBody = serde_json::from_str(raw).unwrap();
    let mut item = checkpoint
        .state
        .transcript
        .get(&ItemId::from_raw("i1"))
        .unwrap()
        .clone();
    item.body = body;
    checkpoint
        .state
        .transcript
        .apply(&Event::ItemUpdated { item });
    let encoded = serde_json::to_string(&checkpoint).unwrap();
    assert!(encoded.contains(raw), "{encoded}");
    assert_eq!(
        serde_json::from_str::<Checkpoint>(&encoded).unwrap(),
        checkpoint
    );
}
