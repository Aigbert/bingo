use super::{command::MemoryCommand, service::Memories, *};
use bingo_sdk::{Command, CommandContext, CommandOutcome, ErrorCode, SessionId};
use bingo_services::memory::{Forget, List, Read, Scope, Service};

#[tokio::test]
async fn commands_and_service_forget_the_same_file_with_the_same_reply() {
    let home = tempfile::tempdir().unwrap();
    let data = home.path().join("data");
    let service = Memories {
        data: data.clone(),
        config: home.path().join("config"),
    };
    let cwd = home.path().join("work");
    std::fs::create_dir(&cwd).unwrap();
    let user = dir::user(&data);
    std::fs::create_dir_all(&user).unwrap();
    let cx = CommandContext {
        session: SessionId::from_raw("s"),
        cwd: cwd.clone(),
        host: bingo_sdk::testing::NoHost::handle(),
        cancel: Default::default(),
    };
    let request = Forget {
        cwd: cwd.clone(),
        scope: Some(Scope::User),
        name: "rule".into(),
    };
    let seed = || {
        std::fs::write(user.join("rule.md"), "malformed but removable").unwrap();
        std::fs::write(user.join("MEMORY.md"), "[rule](rule.md)\nkeep\n").unwrap();
    };
    seed();
    assert_reads(&service, &cwd, &user).await;
    let reply = service.forget(request).await.unwrap();
    seed();
    let outcome = MemoryCommand::new(data)
        .run("forget user rule", &cx)
        .await
        .unwrap();
    assert_eq!(
        outcome,
        CommandOutcome::Applied {
            message: Some(reply)
        }
    );
    assert!(!user.join("rule.md").exists());
    assert_eq!(
        std::fs::read_to_string(user.join("MEMORY.md")).unwrap(),
        "keep\n"
    );
}
async fn assert_reads(service: &Memories, cwd: &std::path::Path, user: &std::path::Path) {
    let rows = service.list(List { cwd: cwd.into() }).await.unwrap();
    assert_eq!(rows.memories.len(), 1);
    assert!(rows.memories[0].error.is_some());
    assert_eq!(
        service
            .read(Read {
                cwd: cwd.into(),
                path: user.join("rule.md")
            })
            .await
            .unwrap(),
        "malformed but removable"
    );
    assert!(
        service
            .read(Read {
                cwd: cwd.into(),
                path: cwd.join("outside")
            })
            .await
            .is_err()
    );
}
#[tokio::test]
async fn an_unscoped_name_in_both_scopes_names_both_and_deletes_neither() {
    let home = tempfile::tempdir().unwrap();
    let data = home.path().join("data");
    let cwd = home.path();
    let user = dir::user(&data);
    let project = project_dir(&data, cwd).await;
    for scope in [&user, &project] {
        std::fs::create_dir_all(scope).unwrap();
        std::fs::write(scope.join("rule.md"), "fact").unwrap();
    }
    let cx = CommandContext {
        session: SessionId::from_raw("s"),
        cwd: cwd.into(),
        host: bingo_sdk::testing::NoHost::handle(),
        cancel: Default::default(),
    };
    let error = MemoryCommand::new(data)
        .run("forget rule", &cx)
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert!(error.message.contains("user") && error.message.contains("project"));
    assert!(user.join("rule.md").exists() && project.join("rule.md").exists());
}
