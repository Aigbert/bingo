//! The nearest team file, one section at a time.
use std::{path::Path, sync::Arc};
use async_trait::async_trait;
use bingo_sdk::{HostHandle, ServiceKey, WireService};
use serde_json::Value;
pub use bingo_sdk::ServiceError as TeamError;

pub static KEY: ServiceKey<dyn TeamFile> = ServiceKey::new("agents.team");
pub const CAPABILITY: &str = "service:agents.team";

#[async_trait]
pub trait TeamFile: Send + Sync {
    async fn section(&self, cwd: &Path, key: &str) -> Result<Value, TeamError>;
}

pub fn wire(_service: Arc<dyn TeamFile>) -> Arc<dyn WireService> { unimplemented!() }
pub fn find(_host: &HostHandle) -> Option<Arc<dyn TeamFile>> { unimplemented!() }

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::testing::ServiceHost;
    struct File;
    fn fixture() -> Value { serde_json::from_str(include_str!("../fixtures/team.json")).unwrap() }
    #[async_trait]
    impl TeamFile for File {
        async fn section(&self, cwd: &Path, key: &str) -> Result<Value, TeamError> {
            assert_eq!(cwd, Path::new("/work/project"));
            assert_eq!(key, "rooms");
            Ok(fixture()["answer"]["section"].clone())
        }
    }
    #[tokio::test]
    async fn the_team_wire_contract_round_trips() {
        let recorded = fixture();
        let face = wire(Arc::new(File));
        assert_eq!(face.call(recorded["method"].as_str().unwrap(), recorded["params"].clone()).await.unwrap(), recorded["answer"]);
        assert!(face.call("roles", Value::Null).await.unwrap_err().to_string().contains("section"));
        assert!(face.call("section", Value::Null).await.is_err());
        let host = ServiceHost::handle();
        host.open_service(KEY.name(), face).unwrap();
        assert_eq!(find(&host).unwrap().section(Path::new("/work/project"), "rooms").await.unwrap(), recorded["answer"]["section"]);
    }
    #[tokio::test]
    async fn a_typed_team_owner_is_found_without_a_wire_face() {
        let owner: Arc<dyn TeamFile> = Arc::new(File);
        let host = ServiceHost::holding(&KEY, owner.clone());
        assert!(Arc::ptr_eq(&find(&host).unwrap(), &owner));
        assert!(find(&ServiceHost::handle()).is_none());
    }
}
