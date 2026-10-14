//! One report per test, suitable for a text line or NDJSON.
use serde::{Deserialize, Serialize};
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub file: String,
    pub test: String,
    pub outcome: Outcome,
}
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum Outcome {
    Pass,
    Fail { message: String, at: String },
    Skip,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reports_round_trip_as_single_json_lines() {
        for outcome in [
            Outcome::Pass,
            Outcome::Fail {
                message: "expected 2\nreceived 1".into(),
                at: "register.test.ts:4".into(),
            },
            Outcome::Skip,
        ] {
            let report = Report {
                file: "register.test.ts".into(),
                test: "counts".into(),
                outcome,
            };
            let line = serde_json::to_string(&report).unwrap();
            assert_eq!(line.lines().count(), 1);
            assert_eq!(serde_json::from_str::<Report>(&line).unwrap(), report);
        }
    }
}

#[async_trait::async_trait]
pub trait Driver: Send {
    fn site(&self) -> Option<serde_json::Value> {
        None
    }
    async fn call(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String>;
}
pub struct Runner {
    engine: std::sync::Arc<crate::engine::Engine>,
    file: String,
}
#[derive(Deserialize)]
struct Case {
    name: String,
    at: String,
}
impl Runner {
    pub async fn load(
        root: &std::path::Path,
        file: &str,
        limits: crate::engine::Limits,
    ) -> Result<Self, String> {
        Ok(Self {
            engine: crate::engine::Engine::test(root, file, limits)
                .await
                .map_err(|e| e.to_string())?,
            file: file.into(),
        })
    }
    pub async fn run(
        &self,
        driver: &mut dyn Driver,
        mut site: serde_json::Value,
    ) -> Result<Vec<Report>, String> {
        site["testing"] = serde_json::json!("list");
        let cases: Vec<Case> = serde_json::from_value(self.dispatch(driver, site.clone()).await?)
            .map_err(|e| e.to_string())?;
        let mut reports = Vec::new();
        for (index, case) in cases.into_iter().enumerate() {
            site = driver.site().unwrap_or(site);
            site["testing"] = serde_json::json!(index);
            let outcome = match self.dispatch(driver, site.clone()).await {
                Ok(value) => serde_json::from_value(value).map_err(|e| e.to_string())?,
                Err(message) => Outcome::Fail {
                    message,
                    at: case.at,
                },
            };
            reports.push(Report {
                file: self.file.clone(),
                test: case.name,
                outcome,
            });
        }
        Ok(reports)
    }
    async fn dispatch(
        &self,
        driver: &mut dyn Driver,
        site: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut call = self
            .engine
            .call("test", serde_json::Value::Null, site)
            .map_err(|e| e.to_string())?;
        loop {
            tokio::select! { biased;
                Some(operation) = call.operations.recv() => {
                    let result = bounded(driver.call(&operation.method, operation.params)).await;
                    let _ = operation.reply.send(result);
                }
                result = &mut call.result => return result.map_err(|e| e.to_string())?.map_err(|e| e.to_string()),
            }
        }
    }
    pub async fn close(&self) {
        self.engine.close().await;
    }
}

mod discover;
pub use discover::imports_testing;
mod environment;
pub(crate) use environment::Started;
pub use environment::{Environment, Paused};

async fn bounded<T>(
    future: impl std::future::Future<Output = Result<T, String>>,
) -> Result<T, String> {
    let (complete, waiting) = std::sync::mpsc::channel::<()>();
    let deadline =
        tokio::task::spawn_blocking(move || waiting.recv_timeout(bingo_sdk::HOOK_DEADLINE));
    let result = tokio::select! {
        result = future => result,
        _ = deadline => Err("test host operation exceeded its wall deadline".into()),
    };
    drop(complete);
    result
}
