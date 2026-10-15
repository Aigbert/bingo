use super::*;
use bingo_sdk::testing::ServiceHost;
struct Owner;
#[async_trait]
impl TasksWrite for Owner {
    async fn board(&self, request: Session) -> Result<Vec<TaskRow>, ServiceError> {
        assert_eq!(request.session.as_str(), "ses_parent");
        Ok(vec![])
    }
    async fn add(&self, request: Add) -> Result<String, ServiceError> {
        assert_eq!(request.session.as_str(), "ses_parent");
        Ok("add accepted".into())
    }
    async fn set_status(&self, request: SetStatus) -> Result<String, ServiceError> {
        assert_eq!(request.session.as_str(), "ses_parent");
        Ok("set_status accepted".into())
    }
    async fn reassign(&self, request: Reassign) -> Result<String, ServiceError> {
        assert_eq!(request.session.as_str(), "ses_parent");
        Ok("reassign accepted".into())
    }
}
#[tokio::test]
async fn native_and_wire_find_preserve_every_fixture() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("../../fixtures/tasks_write/verbs.json")).unwrap();
    let owner: Arc<dyn TasksWrite> = Arc::new(Owner);
    let native = ServiceHost::holding(&KEY, owner.clone());
    assert!(Arc::ptr_eq(&find(&native).unwrap(), &owner));
    assert!(find(&ServiceHost::handle()).is_none());
    let adapter = wire(owner);
    let host = ServiceHost::handle();
    host.open_service(KEY.name(), adapter.clone()).unwrap();
    let roundtrip = wire(find(&host).unwrap());
    for fixture in fixtures {
        let method = fixture["method"].as_str().unwrap();
        for face in [&adapter, &roundtrip] {
            assert_eq!(
                face.call(method, fixture["params"].clone()).await.unwrap(),
                fixture["answer"]
            );
            assert!(face.call(method, Value::Null).await.is_err());
        }
    }
    assert!(adapter.call("unknown", Value::Null).await.is_err());
    let face = adapter.face().unwrap();
    assert!(face.methods.iter().all(|m| m.entry.pinned == ["/session"]));
}
