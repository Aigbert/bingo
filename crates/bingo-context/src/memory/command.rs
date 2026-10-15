//! `/memory`: what is remembered, in both scopes, for the person who cannot
//! read the prompt.
//!
//! Listing and forgetting share the store with the panel. The model remains
//! the only author.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use bingo_sdk::{ArgSpec, Command, CommandContext, CommandOutcome, CommandSpec, KernelError, View};

use crate::memory::file::Memory;
use crate::memory::{dir, project_dir, store};

const HEADERS: [&str; 4] = ["scope", "name", "type", "description"];

#[derive(Debug, Clone)]
pub struct MemoryCommand {
    data_dir: PathBuf,
}

impl MemoryCommand {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }
}

#[async_trait]
impl Command for MemoryCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            keys: vec![],
            hidden: false,
            name: "memory".into(),
            aliases: Vec::new(),
            hint: "what is remembered, about you and about this project".into(),
            args: ArgSpec::None,
            // Reading two directories touches nothing a turn is using.
            instant: true,
            family: "memory".into(),
        }
    }

    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        if !args.trim().is_empty() {
            let (scope, name) = forget_args(args)?;
            use bingo_services::memory::Service;
            let message = super::service::Memories {
                data: self.data_dir.clone(),
                config: PathBuf::new(),
            }
            .forget(bingo_services::memory::Forget {
                cwd: cx.cwd.clone(),
                scope,
                name,
            })
            .await?;
            return Ok(CommandOutcome::Applied {
                message: Some(message),
            });
        }
        let user = dir::user(&self.data_dir);
        let project = project_dir(&self.data_dir, &cx.cwd).await;
        let mut listed = rows("user", &store::list(&user).await);
        listed.extend(rows("project", &store::list(&project).await));
        if listed.is_empty() {
            return Ok(CommandOutcome::Applied {
                message: Some(nothing(&user, &project)),
            });
        }
        Ok(CommandOutcome::View {
            view: View::Table {
                headers: HEADERS.map(str::to_string).to_vec(),
                rows: listed,
            },
        })
    }
}

fn forget_args(args: &str) -> Result<(Option<bingo_services::memory::Scope>, String), KernelError> {
    use bingo_services::memory::Scope;
    let words = args.split_whitespace().collect::<Vec<_>>();
    let (scope, name) = match words.as_slice() {
        ["forget", name] => (None, *name),
        ["forget", "user", name] => (Some(Scope::User), *name),
        ["forget", "project", name] => (Some(Scope::Project), *name),
        _ => {
            return Err(KernelError::new(
                bingo_sdk::ErrorCode::InvalidInput,
                "usage: /memory forget [user|project] <name>",
            ));
        }
    };
    Ok((scope, name.into()))
}

/// One scope's memories, in the columns the model reads them by.
fn rows(scope: &str, memories: &[Memory]) -> Vec<Vec<String>> {
    memories
        .iter()
        .map(|memory| {
            vec![
                scope.to_string(),
                memory.name.clone(),
                memory.kind.as_str().to_string(),
                memory.description.clone(),
            ]
        })
        .collect()
}

/// An empty memory says where memories go, because that is the only thing
/// worth knowing about one.
fn nothing(user: &Path, project: &Path) -> String {
    format!(
        "nothing is remembered yet; memories go in {} and {}",
        user.display(),
        project.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::file::Kind;
    use bingo_sdk::SessionId;

    struct Person {
        data: tempfile::TempDir,
        cwd: tempfile::TempDir,
    }

    impl Person {
        fn new() -> Self {
            Self {
                data: tempfile::tempdir().expect("a data dir"),
                cwd: tempfile::tempdir().expect("a cwd"),
            }
        }

        fn context(&self) -> CommandContext {
            CommandContext {
                cancel: bingo_sdk::CancellationToken::new(),
                session: SessionId::from_raw("ses_test"),
                cwd: self.cwd.path().to_path_buf(),
                host: bingo_sdk::testing::NoHost::handle(),
            }
        }

        async fn project(&self) -> PathBuf {
            project_dir(self.data.path(), self.cwd.path()).await
        }

        async fn remember(&self, at: &Path, name: &str, kind: Kind) {
            store::save(
                at,
                &Memory {
                    name: name.into(),
                    description: format!("what {name} is about"),
                    kind,
                    body: "the fact\n".into(),
                },
            )
            .await
            .expect("a memory");
        }

        async fn run(&self) -> CommandOutcome {
            MemoryCommand::new(self.data.path().to_path_buf())
                .run("", &self.context())
                .await
                .expect("a listing")
        }
    }

    #[test]
    fn the_spec_runs_now_and_takes_nothing() {
        let spec = MemoryCommand::new(PathBuf::from("/data")).spec();
        assert_eq!(spec.name, "memory");
        assert_eq!(spec.args, ArgSpec::None);
        assert!(spec.instant, "reading two directories never waits");
        assert_eq!(spec.family, "memory");
    }

    #[tokio::test]
    async fn it_lists_both_scopes() {
        let person = Person::new();
        let user = dir::user(person.data.path());
        person
            .remember(&user, "prefers-short-replies", Kind::User)
            .await;
        let project = person.project().await;
        person
            .remember(&project, "the-build-runs-cargo-test", Kind::Project)
            .await;

        let CommandOutcome::View {
            view: View::Table { headers, rows },
        } = person.run().await
        else {
            panic!("a memory is a table");
        };
        assert_eq!(headers, HEADERS);
        assert_eq!(
            rows,
            [
                vec![
                    "user".to_string(),
                    "prefers-short-replies".into(),
                    "user".into(),
                    "what prefers-short-replies is about".into(),
                ],
                vec![
                    "project".to_string(),
                    "the-build-runs-cargo-test".into(),
                    "project".into(),
                    "what the-build-runs-cargo-test is about".into(),
                ],
            ]
        );
    }

    #[tokio::test]
    async fn an_empty_memory_says_where_memories_go() {
        let person = Person::new();
        let CommandOutcome::Applied { message } = person.run().await else {
            panic!("nothing to table");
        };
        let message = message.expect("a message");
        assert!(message.contains("memory"), "{message}");
        assert!(
            message.contains(&dir::user(person.data.path()).display().to_string()),
            "{message}"
        );
    }
}
