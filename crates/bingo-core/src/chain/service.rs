//! Only the wire face raises noun events; typed service calls stay direct.
use super::*;
use serde_json::Value;
use std::sync::Weak;

pub(crate) struct Wire {
    pub host: Weak<crate::host::Host>,
    pub owner: Arc<dyn WireService>,
    pub face: Face,
}
#[async_trait::async_trait]
impl WireService for Wire {
    fn face(&self) -> Option<Face> {
        Some(self.face.clone())
    }
    async fn call(&self, method: &str, params: Value) -> Result<Value, ServiceError> {
        let Some(named) = self.face.find(method) else {
            return self.owner.call(method, params).await;
        };
        let host = self
            .host
            .upgrade()
            .ok_or_else(|| ServiceError("the host is gone".into()))?;
        let cx = host.catalogue_context(None);
        let report = Report::default();
        let result = json::dispatch(
            &named.entry,
            &host.hook_set().links().await,
            &cx,
            params,
            Next::new(|params| {
                Box::pin(async move {
                    self.owner
                        .call(method, params)
                        .await
                        .map_err(|error| KernelError::new(ErrorCode::Internal, error.to_string()))
                })
            }),
            report.clone(),
        )
        .await;
        for notice in report.take() {
            if let ItemBody::Notice { level, code, text } = notice {
                let _ = host.notice(level, &code, &text).await;
            }
        }
        result.map_err(|error| ServiceError(error.message))
    }
}
