use super::{dir, file, forget, project_dir, store};
use async_trait::async_trait;
use bingo_sdk::{Contribution, ErrorCode, KernelError};
use bingo_services::memory::{self, Forget, Kind, List, Listing, MemoryRow, Read, Scope, Service};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct Memories {
    pub data: PathBuf,
    pub config: PathBuf,
}
impl Memories {
    async fn scope(&self, cwd: &Path, scope: &Scope) -> PathBuf {
        match scope {
            Scope::User => dir::user(&self.data),
            Scope::Project => project_dir(&self.data, cwd).await,
        }
    }
    async fn can_read(&self, cwd: &Path, path: &Path) -> bool {
        for scope in [Scope::User, Scope::Project] {
            if path.parent() == Some(self.scope(cwd, &scope).await.as_path()) {
                let valid = path.extension().is_some_and(|extension| extension == "md")
                    && path
                        .file_stem()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| forget::valid_name(name).is_ok());
                return valid
                    && tokio::fs::symlink_metadata(path)
                        .await
                        .is_ok_and(|metadata| metadata.is_file());
            }
        }
        crate::InstructionsContributor::new(self.config.clone())
            .loaded(cwd)
            .await
            .iter()
            .any(|row| row.path == path)
    }
    async fn resolve(&self, input: &Forget) -> Result<Scope, KernelError> {
        forget::valid_name(&input.name)?;
        if let Some(scope) = &input.scope {
            return Ok(scope.clone());
        }
        let user = tokio::fs::try_exists(
            self.scope(&input.cwd, &Scope::User)
                .await
                .join(format!("{}.md", input.name)),
        )
        .await
        .map_err(forget::io)?;
        let project = tokio::fs::try_exists(
            self.scope(&input.cwd, &Scope::Project)
                .await
                .join(format!("{}.md", input.name)),
        )
        .await
        .map_err(forget::io)?;
        match (user, project) {
            (true, true) => Err(KernelError::new(
                ErrorCode::InvalidInput,
                format!(
                    "{} exists in both user and project scopes; specify user or project",
                    input.name
                ),
            )),
            (true, false) => Ok(Scope::User),
            (false, true) => Ok(Scope::Project),
            _ => Err(KernelError::new(
                ErrorCode::NotFound,
                format!("memory {} was not found", input.name),
            )),
        }
    }
}
#[async_trait]
impl Service for Memories {
    async fn list(&self, input: List) -> Result<Listing, KernelError> {
        let mut memories = Vec::new();
        for scope in [Scope::Project, Scope::User] {
            memories.extend(list_scope(&self.scope(&input.cwd, &scope).await, scope).await?);
        }
        let instructions = crate::InstructionsContributor::new(self.config.clone())
            .loaded(&input.cwd)
            .await;
        Ok(Listing {
            memories,
            instructions,
        })
    }
    async fn forget(&self, input: Forget) -> Result<String, KernelError> {
        let scope = self.resolve(&input).await?;
        let directory = self.scope(&input.cwd, &scope).await;
        let name = input.name.clone();
        tokio::task::spawn_blocking(move || store::forget(&directory, &name))
            .await
            .map_err(|error| KernelError::new(ErrorCode::Internal, error.to_string()))??;
        Ok(format!(
            "forgot {} memory \"{}\"; this session's prompt may keep it until next composed. Say so in chat if it should not be learned again.",
            scope.as_str(),
            input.name
        ))
    }
    async fn read(&self, input: Read) -> Result<String, KernelError> {
        if !self.can_read(&input.cwd, &input.path).await {
            return Err(KernelError::new(
                ErrorCode::NotFound,
                "that file is no longer listed",
            ));
        }
        tokio::fs::read_to_string(input.path)
            .await
            .map_err(forget::io)
    }
}
async fn list_scope(path: &Path, scope: Scope) -> Result<Vec<MemoryRow>, KernelError> {
    let mut entries = match tokio::fs::read_dir(path).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(error) => return Err(forget::io(error)),
    };
    let index = store::index_text(path).await;
    let (carried, _) = crate::files::capped(&index, super::INDEX_LINES, crate::files::MAX_BYTES);
    let mut rows = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(forget::io)? {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "md")
            || path.file_name().is_some_and(|name| name == "MEMORY.md")
        {
            continue;
        }
        if !entry.file_type().await.map_err(forget::io)?.is_file() {
            continue;
        }
        rows.push(row(path, scope.clone(), &carried).await);
    }
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(rows)
}
async fn row(path: PathBuf, scope: Scope, carried: &str) -> MemoryRow {
    let name = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let read = tokio::fs::read_to_string(&path).await;
    let tokens = carried
        .split_inclusive('\n')
        .filter(|line| super::index::targets(line, &format!("{name}.md")))
        .map(bingo_sdk::tokens::text)
        .sum();
    let parsed = read
        .map_err(|e| e.to_string())
        .and_then(|text| file::parse(&name, &text).map_err(|e| e.to_string()));
    let (kind, description, error) = match parsed {
        Ok(memory) => (
            match memory.kind {
                file::Kind::User => Kind::User,
                file::Kind::Feedback => Kind::Feedback,
                file::Kind::Project => Kind::Project,
                file::Kind::Reference => Kind::Reference,
            },
            memory.description,
            None,
        ),
        Err(error) => (Kind::Malformed, String::new(), Some(error)),
    };
    MemoryRow {
        scope,
        name,
        kind,
        description,
        path,
        tokens,
        error,
    }
}
pub fn contribution(data: PathBuf, config: PathBuf) -> Contribution {
    let service: Arc<dyn Service> = Arc::new(Memories { data, config });
    Contribution::service(&memory::KEY, service.clone(), Some(memory::wire(service)))
}
