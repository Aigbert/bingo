use bingo_sdk::*;
use bingo_services::agents_control::{AgentRow, AgentsControl, Message, Named, Session};

#[async_trait::async_trait]
impl AgentsControl for crate::service::AgentService {
    async fn roster(&self, request: Session) -> Result<Vec<AgentRow>, ServiceError> {
        roster(&self.host()?, &request.session)
            .await
            .map_err(failed)
    }
    async fn stop(&self, request: Named) -> Result<String, ServiceError> {
        crate::stop::stop(&self.host()?, &request.session, &request.name)
            .await
            .map_err(failed)
    }
    async fn dismiss(&self, request: Named) -> Result<String, ServiceError> {
        crate::dismiss::dismiss(&self.host()?, &request.session, &request.name)
            .await
            .map_err(failed)
    }
    async fn message(&self, request: Message) -> Result<String, ServiceError> {
        crate::message::send(
            &self.host()?,
            &request.session,
            &request.name,
            &request.text,
            false,
        )
        .await
        .map_err(failed)
    }
    async fn reopen(&self, request: Message) -> Result<String, ServiceError> {
        crate::message::send(
            &self.host()?,
            &request.session,
            &request.name,
            &request.text,
            true,
        )
        .await
        .map_err(failed)
    }
}
pub async fn roster(host: &HostHandle, session: &SessionId) -> Result<Vec<AgentRow>, KernelError> {
    Ok(crate::names::agents(host, session)
        .await?
        .into_iter()
        .map(|summary| AgentRow { summary })
        .collect())
}
fn failed(error: KernelError) -> ServiceError {
    ServiceError(error.message)
}
