//! The nearest team file, one section at a time.
use async_trait::async_trait;
pub use bingo_sdk::ServiceError as TeamError;
use bingo_sdk::{HostHandle, ServiceKey, WireService};
use serde_json::Value;
use std::{path::Path, sync::Arc};

pub static KEY: ServiceKey<dyn TeamFile> = ServiceKey::new("agents.team");
pub const CAPABILITY: &str = "service:agents.team";

#[async_trait]
pub trait TeamFile: Send + Sync {
    async fn section(&self, cwd: &Path, key: &str) -> Result<Value, TeamError>;
}

pub fn wire(service: Arc<dyn TeamFile>) -> Arc<dyn WireService> {
    Arc::new(ToWire(service))
}

pub fn find(host: &HostHandle) -> Option<Arc<dyn TeamFile>> {
    host.service(&KEY).or_else(|| {
        host.service_wire(KEY.name())
            .map(|wire| Arc::new(FromWire(wire)) as Arc<dyn TeamFile>)
    })
}

struct ToWire(Arc<dyn TeamFile>);

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct Asked {
    cwd: std::path::PathBuf,
    key: String,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct Answer {
    section: Value,
}

#[async_trait]
impl WireService for ToWire {
    fn face(&self) -> Option<bingo_sdk::Face> {
        Some(bingo_sdk::Face::new("team").method::<Asked, Answer>("section", false))
    }

    async fn call(&self, method: &str, params: Value) -> Result<Value, TeamError> {
        if method != "section" {
            return Err(TeamError(format!(
                "{} speaks `section`, not `{method}`",
                KEY.name()
            )));
        }
        let asked: Asked =
            serde_json::from_value(params).map_err(|error| TeamError(error.to_string()))?;
        let section = self.0.section(&asked.cwd, &asked.key).await?;
        Ok(serde_json::json!({ "section": section }))
    }
}

struct FromWire(Arc<dyn WireService>);

#[async_trait]
impl TeamFile for FromWire {
    async fn section(&self, cwd: &Path, key: &str) -> Result<Value, TeamError> {
        let cwd = cwd.to_str().ok_or_else(|| {
            TeamError(format!(
                "{} is not a directory name this can ask about",
                cwd.display()
            ))
        })?;
        let answer = self
            .0
            .call("section", serde_json::json!({ "cwd": cwd, "key": key }))
            .await?;
        Ok(answer["section"].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::testing::ServiceHost;
    struct File;
    fn fixture() -> Value {
        serde_json::from_str(include_str!("../fixtures/team.json")).unwrap()
    }
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
        assert_eq!(
            face.call(
                recorded["method"].as_str().unwrap(),
                recorded["params"].clone()
            )
            .await
            .unwrap(),
            recorded["answer"]
        );
        assert!(
            face.call("roles", Value::Null)
                .await
                .unwrap_err()
                .to_string()
                .contains("section")
        );
        assert!(face.call("section", Value::Null).await.is_err());
        let host = ServiceHost::handle();
        host.open_service(KEY.name(), face).unwrap();
        assert_eq!(
            find(&host)
                .unwrap()
                .section(Path::new("/work/project"), "rooms")
                .await
                .unwrap(),
            recorded["answer"]["section"]
        );
    }
    #[tokio::test]
    async fn a_typed_team_owner_is_found_without_a_wire_face() {
        let owner: Arc<dyn TeamFile> = Arc::new(File);
        let host = ServiceHost::holding(&KEY, owner.clone());
        assert!(Arc::ptr_eq(&find(&host).unwrap(), &owner));
        assert!(find(&ServiceHost::handle()).is_none());
    }
}
