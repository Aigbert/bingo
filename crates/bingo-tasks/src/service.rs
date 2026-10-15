use bingo_sdk::*;
use bingo_services::tasks_write::*;
use std::sync::{OnceLock, Weak};

#[derive(Default)]
pub struct Writer(OnceLock<Weak<dyn HostApi>>);
impl Writer {
    pub fn bind(&self, host: &HostHandle) {
        let _ = self.0.set(std::sync::Arc::downgrade(&host.0));
    }
    fn host(&self) -> Result<HostHandle, ServiceError> {
        self.0
            .get()
            .and_then(Weak::upgrade)
            .map(HostHandle)
            .ok_or_else(|| ServiceError("tasks have not started".into()))
    }
}
#[async_trait::async_trait]
impl TasksWrite for Writer {
    async fn board(&self, request: Session) -> Result<Vec<TaskRow>, ServiceError> {
        rows(&self.host()?, &request.session).await.map_err(failed)
    }
    async fn add(&self, request: Add) -> Result<String, ServiceError> {
        crate::create::add(
            &self.host()?,
            &request.session,
            crate::Draft {
                subject: request.subject,
                owner: request.owner,
                ..Default::default()
            },
        )
        .await
        .map_err(failed)
    }
    async fn set_status(&self, request: SetStatus) -> Result<String, ServiceError> {
        crate::update::change(
            &self.host()?,
            &request.session,
            crate::Change {
                id: request.id,
                status: Some(status(request.status)),
                ..Default::default()
            },
            false,
        )
        .await
        .map_err(failed)
    }
    async fn reassign(&self, request: Reassign) -> Result<String, ServiceError> {
        crate::update::assign(&self.host()?, &request.session, request.id, &request.owner)
            .await
            .map_err(failed)
    }
}
fn failed(error: KernelError) -> ServiceError {
    ServiceError(error.message)
}
fn status(status: Status) -> crate::Status {
    match status {
        Status::Pending => crate::Status::Pending,
        Status::InProgress => crate::Status::InProgress,
        Status::Completed => crate::Status::Completed,
    }
}
pub async fn rows(host: &HostHandle, session: &SessionId) -> Result<Vec<TaskRow>, KernelError> {
    Ok(crate::journal::read(host, session)
        .await?
        .into_iter()
        .map(|row| TaskRow {
            id: row.id,
            subject: row.subject,
            owner: row.owner,
            status: match row.status {
                crate::Status::Pending => Status::Pending,
                crate::Status::InProgress => Status::InProgress,
                crate::Status::Completed => Status::Completed,
            },
        })
        .collect())
}
