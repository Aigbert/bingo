use crate::store::Library;
use async_trait::async_trait;
use bingo_sdk::{Contribution, ErrorCode, KernelError};
use bingo_services::experience::{self, EntryRow, Forget, List, Listing, Read, Service};
use std::{path::Path, sync::Arc};

pub struct Experiences(pub Arc<Library>);
fn error(error: std::io::Error) -> KernelError {
    KernelError::new(
        if error.kind() == std::io::ErrorKind::NotFound {
            ErrorCode::NotFound
        } else {
            ErrorCode::Storage
        },
        error.to_string(),
    )
}
fn valid(id: &str) -> Result<(), KernelError> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    {
        return Err(KernelError::new(
            ErrorCode::InvalidInput,
            "expected an experience id",
        ));
    }
    Ok(())
}
pub fn forget(library: &Library, cwd: &Path, id: &str) -> Result<String, KernelError> {
    valid(id)?;
    if !library.path(cwd, id).is_file() {
        return Err(KernelError::new(
            ErrorCode::NotFound,
            format!("experience {id} was not found"),
        ));
    }
    library.delete(cwd, id).map_err(error)?;
    Ok(format!("forgot experience {id}"))
}
#[async_trait]
impl Service for Experiences {
    async fn list(&self, input: List) -> Result<Listing, KernelError> {
        Ok(self
            .0
            .load(&input.cwd)
            .entries
            .iter()
            .map(|entry| EntryRow {
                id: entry.id.clone(),
                status: entry.status.as_str().into(),
                summary: entry.summary.clone(),
                outcomes: format!("{} ok · {} harmful", entry.helpful(), entry.harmful()),
                tokens: bingo_sdk::tokens::text(&crate::frontmatter::to_markdown(entry)),
            })
            .collect())
    }
    async fn forget(&self, input: Forget) -> Result<String, KernelError> {
        forget(&self.0, &input.cwd, &input.id)
    }
    async fn read(&self, input: Read) -> Result<String, KernelError> {
        valid(&input.id)?;
        std::fs::read_to_string(self.0.path(&input.cwd, &input.id)).map_err(error)
    }
}
pub fn contribution(library: Arc<Library>) -> Contribution {
    let service: Arc<dyn Service> = Arc::new(Experiences(library));
    Contribution::service(
        &experience::KEY,
        service.clone(),
        Some(experience::wire(service)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::{Command, CommandOutcome};
    #[tokio::test]
    async fn command_and_service_share_the_reply_and_delete() {
        let fixture = crate::tests::Fixture::new();
        let entry = crate::entry::tests::entry();
        fixture.library.save(&fixture.cwd(), &entry).unwrap();
        let service = Experiences(fixture.library.clone());
        let listing = service.list(List { cwd: fixture.cwd() }).await.unwrap();
        assert_eq!(listing[0].id, entry.id);
        assert!(listing[0].tokens > 0);
        assert!(
            service
                .read(Read {
                    cwd: fixture.cwd(),
                    id: entry.id.clone()
                })
                .await
                .unwrap()
                .contains(&entry.summary)
        );
        let reply = service
            .forget(Forget {
                cwd: fixture.cwd(),
                id: entry.id.clone(),
            })
            .await
            .unwrap();
        fixture.library.save(&fixture.cwd(), &entry).unwrap();
        let outcome = crate::ExperienceCommand::new(fixture.library.clone())
            .run(&format!("forget {}", entry.id), &fixture.command())
            .await
            .unwrap();
        assert_eq!(
            outcome,
            CommandOutcome::Applied {
                message: Some(reply)
            }
        );
        assert!(!fixture.library.path(&fixture.cwd(), &entry.id).exists());
        assert_eq!(
            service
                .forget(Forget {
                    cwd: fixture.cwd(),
                    id: "../outside".into()
                })
                .await
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
}
