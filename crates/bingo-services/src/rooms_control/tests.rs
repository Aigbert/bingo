use super::*;
use bingo_sdk::testing::ServiceHost;
struct Owner;
#[async_trait]
impl RoomsControl for Owner {
    async fn rooms(&self, request: Session) -> Result<Vec<RoomRow>, ServiceError> {
        assert_eq!(request.session.as_str(), "ses_parent");
        Ok(vec![])
    }
    async fn seat(&self, request: Seat) -> Result<String, ServiceError> {
        assert_eq!(request.session.as_str(), "ses_parent");
        Ok("seat accepted".into())
    }
    async fn unseat(&self, request: Seat) -> Result<String, ServiceError> {
        assert_eq!(request.session.as_str(), "ses_parent");
        Ok("unseat accepted".into())
    }
    async fn set_purpose(&self, request: Purpose) -> Result<String, ServiceError> {
        assert_eq!(request.session.as_str(), "ses_parent");
        Ok("set_purpose accepted".into())
    }
    async fn close(&self, request: Named) -> Result<String, ServiceError> {
        assert_eq!(request.session.as_str(), "ses_parent");
        Ok("close accepted".into())
    }
}
#[tokio::test]
async fn native_and_wire_find_preserve_every_fixture() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("../../fixtures/rooms_control/verbs.json")).unwrap();
    let owner: Arc<dyn RoomsControl> = Arc::new(Owner);
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

#[test]
fn a_room_row_names_its_holder_and_identifies_its_journal() {
    let expected: Value =
        serde_json::from_str(include_str!("../../fixtures/rooms_control/row.json")).unwrap();
    let row: RoomRow = serde_json::from_value(expected.clone()).unwrap();
    assert_eq!(serde_json::to_value(row).unwrap(), expected);
}
