use super::*;
use bingo_sdk::{Preview, Tool};
use serde_json::json;

#[test]
fn only_an_explicit_force_push_of_one_branch_has_stakes() {
    for command in [
        "git push --force origin dev",
        "git push origin dev -f",
        "git push --force-with-lease -- origin 'feature/topic'",
    ] {
        assert!(force_push(command).is_some(), "{command}");
    }
    for command in [
        "git push origin dev",
        "git push --force",
        "git push --force origin",
        "git push --force --all origin",
        "git push --force origin dev main",
        "git push --force origin dev:main",
        "git push --force origin +dev",
        "git -C elsewhere push --force origin dev",
        "git push --force origin dev; echo x",
        "git push --force origin $(echo dev)",
        "git push --force origin dev && true",
        "git push --force https://example.com/repo dev",
        "echo 'git push --force origin dev'",
        "git push --force origin ../dev",
        "git push --force origin dev > out",
    ] {
        assert_eq!(force_push(command), None, "{command}");
    }
}

fn git(cwd: &Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", cwd.join("no-config"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-b", "dev"]);
    git(dir.path(), &["config", "user.name", "Fixture"]);
    git(
        dir.path(),
        &["config", "user.email", "fixture@example.invalid"],
    );
    git(dir.path(), &["config", "commit.gpgSign", "false"]);
    git(dir.path(), &["commit", "--allow-empty", "-m", "base"]);
    git(dir.path(), &["branch", "remote-work"]);
    git(dir.path(), &["checkout", "remote-work"]);
    dir
}

fn add_remote_commit(cwd: &Path, subject: &str) {
    git(cwd, &["commit", "--allow-empty", "-m", subject]);
    git(cwd, &["update-ref", "refs/remotes/origin/dev", "HEAD"]);
}

fn tool_stakes(cwd: &Path, command: &str) -> Option<Stakes> {
    let preview = crate::tests::tool()
        .2
        .preview(&json!({"command":command}), cwd)
        .unwrap();
    let Preview::Command { stakes, .. } = preview else {
        panic!("command preview")
    };
    stakes
}

#[test]
fn force_push_stakes_count_all_commits_and_keep_the_first_five() {
    let dir = repository();
    for n in 1..=7 {
        add_remote_commit(dir.path(), &format!("remote work {n}"));
    }
    let expected = git(dir.path(), &["log", "--oneline", "dev..origin/dev"]);
    let stakes = tool_stakes(dir.path(), "git push --force origin dev").unwrap();
    assert_eq!(stakes.line, "drops 7 commits that only origin/dev has");
    assert_eq!(stakes.items, expected.lines().take(5).collect::<Vec<_>>());
    assert_eq!(tool_stakes(dir.path(), "git push origin dev"), None);
}

#[tokio::test]
async fn a_preview_inside_a_runtime_names_one_commit_and_zero_commits() {
    let dir = repository();
    add_remote_commit(dir.path(), "remote work");
    let stakes = tool_stakes(dir.path(), "git push -f origin dev").unwrap();
    assert_eq!(stakes.line, "drops 1 commit that only origin/dev has");
    assert_eq!(stakes.items.len(), 1);
    git(dir.path(), &["update-ref", "refs/heads/dev", "HEAD"]);
    let stakes = tool_stakes(dir.path(), "git push -f origin dev").unwrap();
    assert_eq!(stakes.line, "drops 0 commits that only origin/dev has");
    assert!(stakes.items.is_empty());
}

#[test]
fn a_failed_git_lookup_leaves_the_preview_without_stakes() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(tool_stakes(dir.path(), "git push --force origin dev"), None);
    let dir = repository();
    assert_eq!(tool_stakes(dir.path(), "git push --force origin dev"), None);
    assert_eq!(
        tool_stakes(&dir.path().join("missing"), "git push --force origin dev"),
        None
    );
}

#[test]
fn missing_objects_in_a_partial_clone_are_never_fetched() {
    let dir = repository();
    add_remote_commit(dir.path(), "remote work");
    let remote = tempfile::tempdir().unwrap();
    git(
        dir.path(),
        &["clone", "--bare", ".", remote.path().to_str().unwrap()],
    );
    git(
        dir.path(),
        &[
            "config",
            "remote.origin.url",
            remote.path().to_str().unwrap(),
        ],
    );
    git(dir.path(), &["config", "remote.origin.promisor", "true"]);
    git(dir.path(), &["config", "core.repositoryformatversion", "1"]);
    git(dir.path(), &["config", "extensions.partialClone", "origin"]);
    let hash = git(dir.path(), &["rev-parse", "refs/remotes/origin/dev"]);
    let hash = hash.trim();
    let object = dir
        .path()
        .join(".git/objects")
        .join(&hash[..2])
        .join(&hash[2..]);
    std::fs::remove_file(&object).unwrap();
    assert_eq!(tool_stakes(dir.path(), "git push --force origin dev"), None);
    assert!(!object.exists(), "preview fetched the missing commit");
    assert_eq!(
        std::fs::read_dir(dir.path().join(".git/objects/pack"))
            .unwrap()
            .count(),
        0
    );
    // The local bare remote can supply it, so failure above proves the fetch was suppressed.
    assert!(git(dir.path(), &["log", "--oneline", "dev..origin/dev"]).contains("remote work"));
}

#[tokio::test]
async fn timed_out_or_failed_output_supplies_no_stakes() {
    for mode in ["slow", "failed"] {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "stakes::tests::log_child", "--nocapture"])
            .env("BINGO_STAKES_TEST_CHILD", mode)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        assert_eq!(
            bounded_log(command, "origin", "dev", Instant::now() + LIMIT).await,
            None
        );
    }
}

#[test]
fn log_child() {
    let Ok(mode) = std::env::var("BINGO_STAKES_TEST_CHILD") else {
        return;
    };
    println!("4e1a9c2 incomplete output");
    if mode == "slow" {
        std::thread::sleep(Duration::from_secs(30));
    }
    std::process::exit(1);
}
