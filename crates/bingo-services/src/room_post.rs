//! The rooms plugin's single serial writer.
use async_trait::async_trait;
use bingo_sdk::{ItemId, ServiceError, ServiceKey, SessionId};
use serde::{Deserialize, Serialize};

pub static KEY: ServiceKey<dyn RoomPost> = ServiceKey::new("rooms.post");
pub const CAPABILITY: &str = "service:rooms.post";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "text", rename_all = "snake_case")]
pub enum Said { Text(String), Again }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "text", rename_all = "snake_case")]
pub enum Posted { Landed(String), Bounced(String) }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub room: SessionId,
    pub caller: SessionId,
    pub author: String,
    pub said: Said,
    pub cut: ItemId,
}

#[async_trait]
pub trait RoomPost: Send + Sync {
    async fn post(&self, request: Request) -> Result<Posted, ServiceError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::testing::ServiceHost;
    use serde_json::Value;
    use std::sync::Arc;
    struct Writer;
    #[async_trait]
    impl RoomPost for Writer {
        async fn post(&self, request: Request) -> Result<Posted, ServiceError> {
            assert_eq!(request.room.as_str(), "ses_room");
            assert_eq!(request.caller.as_str(), "ses_scout");
            assert_eq!(request.author, "Scout");
            assert_eq!(request.cut.as_str(), "itm_call");
            Ok(match request.said {
                Said::Text(text) => Posted::Landed(text),
                Said::Again => Posted::Bounced("nothing to repeat".into()),
            })
        }
    }
    #[tokio::test]
    async fn both_service_faces_keep_the_golden_post_contract() {
        for fixture in [include_str!("../fixtures/room_post/text.json"), include_str!("../fixtures/room_post/again.json")] {
            let fixture: Value = serde_json::from_str(fixture).unwrap();
            let wire = wire(Arc::new(Writer));
            assert_eq!(wire.call("post", fixture["params"].clone()).await.unwrap(), fixture["answer"]);
            assert!(wire.call("unknown", Value::Null).await.is_err());
            assert!(wire.call("post", Value::Null).await.is_err());
            let host = ServiceHost::handle();
            host.open_service(KEY.name(), wire).unwrap();
            let request: Request = serde_json::from_value(fixture["params"].clone()).unwrap();
            let reply = find(&host).unwrap().post(request).await.unwrap();
            assert_eq!(serde_json::to_value(reply).unwrap(), fixture["answer"]);
        }
        let owner: Arc<dyn RoomPost> = Arc::new(Writer);
        let host = ServiceHost::holding(&KEY, owner.clone());
        assert!(Arc::ptr_eq(&find(&host).unwrap(), &owner));
        assert!(find(&ServiceHost::handle()).is_none());
    }
}
