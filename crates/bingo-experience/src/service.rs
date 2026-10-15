use crate::store::Library;
use async_trait::async_trait;
use bingo_sdk::{Contribution, ErrorCode, KernelError};
use bingo_services::experience::{
    self, EntryRow, Forget, List, Listing, Outcome, Read, Service, Status,
};
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
fn forget_file(library: &Library, cwd: &Path, id: &str) -> Result<String, KernelError> {
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
pub async fn forget(
    library: Arc<Library>,
    cwd: std::path::PathBuf,
    id: String,
) -> Result<String, KernelError> {
    tokio::task::spawn_blocking(move || forget_file(&library, &cwd, &id))
        .await
        .map_err(join_error)?
}
fn join_error(error: tokio::task::JoinError) -> KernelError {
    KernelError::new(ErrorCode::Internal, error.to_string())
}
fn listing(library: &Library, cwd: &Path) -> Listing {
    use crate::store::ExperienceShelf;
    let shelf = library.load(cwd);
    let carried = crate::render::by_worth(shelf.active());
    let mut rows = shelf
        .entries
        .iter()
        .map(|entry| {
            let tokens = if carried
                .iter()
                .take(crate::contributor::index::MAX)
                .any(|other| other.id == entry.id)
            {
                bingo_sdk::tokens::text(&format!("- {}", crate::render::line(entry)))
            } else {
                0
            };
            EntryRow {
                id: entry.id.clone(),
                status: match entry.status {
                    crate::entry::Status::Active => Status::Active,
                    crate::entry::Status::Retired => Status::Retired,
                },
                summary: entry.summary.clone(),
                outcomes: entry
                    .outcomes
                    .iter()
                    .map(|record| match record.outcome {
                        crate::entry::Outcome::Helpful => Outcome::Helpful,
                        crate::entry::Outcome::Harmful => Outcome::Harmful,
                    })
                    .collect(),
                error: None,
                tokens,
            }
        })
        .collect::<Vec<_>>();
    rows.extend(shelf.unreadable.into_iter().map(|entry| {
        EntryRow {
            id: entry
                .file
                .strip_suffix(".md")
                .unwrap_or(&entry.file)
                .to_owned(),
            status: Status::Unreadable,
            summary: String::new(),
            outcomes: vec![],
            tokens: 0,
            error: Some(entry.why),
        }
    }));
    rows
}
#[async_trait]
impl Service for Experiences {
    async fn list(&self, input: List) -> Result<Listing, KernelError> {
        let library = self.0.clone();
        tokio::task::spawn_blocking(move || listing(&library, &input.cwd))
            .await
            .map_err(join_error)
    }
    async fn forget(&self, input: Forget) -> Result<String, KernelError> {
        forget(self.0.clone(), input.cwd, input.id).await
    }
    async fn read(&self, input: Read) -> Result<String, KernelError> {
        valid(&input.id)?;
        let library = self.0.clone();
        tokio::task::spawn_blocking(move || {
            std::fs::read_to_string(library.path(&input.cwd, &input.id)).map_err(error)
        })
        .await
        .map_err(join_error)?
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

#[cfg(test)]
mod measurement_tests {
    use super::*;
    #[tokio::test]
    async fn bodies_do_not_count_and_unreadable_entries_remain_visible() {
        let fixture = crate::tests::Fixture::new();
        let mut entry = crate::entry::tests::entry();
        fixture.library.save(&fixture.cwd(), &entry).unwrap();
        let service = Experiences(fixture.library.clone());
        let list = || List { cwd: fixture.cwd() };
        let expected = bingo_sdk::tokens::text(&format!("- {}", crate::render::line(&entry)));
        assert_eq!(service.list(list()).await.unwrap()[0].tokens, expected);
        entry.notes = "long body ".repeat(10000);
        fixture.library.save(&fixture.cwd(), &entry).unwrap();
        assert_eq!(service.list(list()).await.unwrap()[0].tokens, expected);
        entry.status = crate::entry::Status::Retired;
        fixture.library.save(&fixture.cwd(), &entry).unwrap();
        std::fs::write(
            fixture.library.dir(&fixture.cwd()).join("broken.md"),
            "not frontmatter",
        )
        .unwrap();
        let rows = service.list(list()).await.unwrap();
        assert_eq!(rows[0].tokens, 0);
        assert_eq!(rows[1].status, Status::Unreadable);
        assert!(rows[1].error.as_ref().unwrap().contains("frontmatter"));
        assert_eq!(rows[1].tokens, 0);
    }
}
