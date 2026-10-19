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
        surface: None,
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
    let outcome = MemoryCommand::new(data, home.path().join("config"))
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
        surface: None,
        session: SessionId::from_raw("s"),
        cwd: cwd.into(),
        host: bingo_sdk::testing::NoHost::handle(),
        cancel: Default::default(),
    };
    let error = MemoryCommand::new(data, home.path().join("config"))
        .run("forget rule", &cx)
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert!(error.message.contains("user") && error.message.contains("project"));
    assert!(user.join("rule.md").exists() && project.join("rule.md").exists());
}

#[tokio::test]
async fn only_carried_index_lines_count_not_the_body_or_omitted_lines() {
    let home = tempfile::tempdir().unwrap();
    let data = home.path().join("data");
    let user = dir::user(&data);
    std::fs::create_dir_all(&user).unwrap();
    let line = "[rule](./rule.md#fact)\n";
    std::fs::write(user.join("MEMORY.md"), line).unwrap();
    std::fs::write(user.join("rule.md"), "short").unwrap();
    let service = Memories {
        data,
        config: home.path().join("config"),
    };
    let list = || List {
        cwd: home.path().into(),
    };
    assert_eq!(
        service.list(list()).await.unwrap().memories[0].tokens,
        bingo_sdk::tokens::text(line)
    );
    std::fs::write(user.join("rule.md"), "long body ".repeat(10000)).unwrap();
    assert_eq!(
        service.list(list()).await.unwrap().memories[0].tokens,
        bingo_sdk::tokens::text(line)
    );
    std::fs::write(
        user.join("MEMORY.md"),
        format!("{line}{}", "other\n".repeat(INDEX_LINES)),
    )
    .unwrap();
    assert_eq!(service.list(list()).await.unwrap().memories[0].tokens, 0);
}

#[tokio::test]
async fn an_index_line_linking_two_memories_is_counted_once() {
    let home = tempfile::tempdir().unwrap();
    let data = home.path().join("data");
    let user = dir::user(&data);
    std::fs::create_dir_all(&user).unwrap();
    let line = "[a](a.md) and [b](b.md)\n";
    std::fs::write(user.join("MEMORY.md"), line).unwrap();
    for name in ["a.md", "b.md"] {
        std::fs::write(user.join(name), "body").unwrap();
    }
    let service = Memories {
        data,
        config: home.path().join("config"),
    };
    let rows = service
        .list(List {
            cwd: home.path().into(),
        })
        .await
        .unwrap()
        .memories;
    assert_eq!(
        rows.iter().map(|row| row.tokens).sum::<u64>(),
        bingo_sdk::tokens::text(line)
    );
}

#[tokio::test]
async fn the_listing_counts_exactly_the_links_the_contributor_carries() {
    let home = tempfile::tempdir().unwrap();
    let data = home.path().join("data");
    let user = dir::user(&data);
    std::fs::create_dir_all(&user).unwrap();
    let total = INDEX_LINES + 10;
    let mut index = String::new();
    for n in 0..total {
        std::fs::write(user.join(format!("fact-{n:03}.md")), "body").unwrap();
        index.push_str(&format!("- [fact {n}](fact-{n:03}.md)\n"));
    }
    std::fs::write(user.join("MEMORY.md"), &index).unwrap();
    let service = Memories {
        data: data.clone(),
        config: home.path().join("config"),
    };
    let rows = service
        .list(List {
            cwd: home.path().into(),
        })
        .await
        .unwrap()
        .memories;
    let blocks = crate::baseline::testing::Journal::at(home.path())
        .contribute(&MemoryContributor::new(data), home.path())
        .await;
    let carried = blocks
        .iter()
        .filter_map(|piece| match piece {
            bingo_sdk::ContextPiece::System(block) => Some(block.text.as_str()),
            bingo_sdk::ContextPiece::User { .. } => None,
        })
        .flat_map(str::lines)
        .filter(|line| line.contains("](fact-"))
        .count();
    assert_eq!(carried, INDEX_LINES);
    assert_eq!(rows.iter().filter(|row| row.tokens > 0).count(), carried);
}
