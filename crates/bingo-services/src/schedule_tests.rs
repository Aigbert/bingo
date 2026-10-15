use super::*;
#[test]
fn fixtures_pin_pending_and_listing() {
    let pending = include_str!("../fixtures/schedule/pending.json");
    let listing = include_str!("../fixtures/schedule/list.json");
    let p: Pending = serde_json::from_str(pending).unwrap();
    let l: Listing = serde_json::from_str(listing).unwrap();
    assert_eq!(json!(p), serde_json::from_str::<Value>(pending).unwrap());
    assert_eq!(json!(l), serde_json::from_str::<Value>(listing).unwrap());
}
struct Owner;
#[async_trait]
impl Service for Owner {
    async fn list(&self) -> Result<Listing, KernelError> {
        Ok(serde_json::from_str(include_str!("../fixtures/schedule/list.json")).unwrap())
    }
    async fn pending(&self, session: SessionId) -> Result<Option<Pending>, KernelError> {
        Ok((session.as_str() == "ses_1").then(|| {
            serde_json::from_str(include_str!("../fixtures/schedule/pending.json")).unwrap()
        }))
    }
    async fn pause(&self, _: String) -> Result<String, KernelError> {
        refused()
    }
    async fn resume(&self, _: String) -> Result<String, KernelError> {
        refused()
    }
    async fn run(&self, _: String) -> Result<String, KernelError> {
        refused()
    }
    async fn forget(&self, _: String) -> Result<String, KernelError> {
        refused()
    }
}
fn refused() -> Result<String, KernelError> {
    assert!(!bingo_sdk::render::asked());
    Err(KernelError::new(
        bingo_sdk::ErrorCode::PermissionDenied,
        "person required",
    ))
}
#[tokio::test]
async fn native_wire_and_mutation_faces() {
    use bingo_sdk::testing::ServiceHost;
    let owner: Arc<dyn Service> = Arc::new(Owner);
    assert!(Arc::ptr_eq(
        &find(&ServiceHost::holding(&KEY, owner.clone())).unwrap(),
        &owner
    ));
    assert!(find(&ServiceHost::handle()).is_none());
    let adapter = wire(owner);
    let host = ServiceHost::handle();
    host.open_service(KEY.name(), adapter.clone()).unwrap();
    let service = find(&host).unwrap();
    assert_eq!(service.list().await.unwrap().entries.len(), 1);
    assert!(
        service
            .pending(SessionId::from_raw("missing"))
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        service
            .pending(SessionId::from_raw("ses_1"))
            .await
            .unwrap()
            .unwrap()
            .note,
        "check"
    );
    for method in ["pause", "resume", "run", "forget"] {
        assert!(adapter.face().unwrap().find(method).unwrap().unsafe_call);
        let fixture: Value =
            serde_json::from_str(include_str!("../fixtures/schedule/mutations.json")).unwrap();
        let answer =
            bingo_sdk::render::under_input(true, adapter.call(method, fixture[method].clone()))
                .await
                .unwrap();
        assert_eq!(answer, fixture["answer"]);
    }
    assert!(adapter.call("unknown", Value::Null).await.is_err());
}
