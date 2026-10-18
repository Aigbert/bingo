//! Consequences available from local refs before a force push is approved.

use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

use bingo_sdk::Stakes;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

const LIMIT: Duration = Duration::from_millis(500);

pub fn preview(command: &str, cwd: &Path) -> Option<Stakes> {
    let (remote, branch) = force_push(command)?;
    let deadline = Instant::now() + LIMIT;
    // Preview is synchronous and can be called from either kind of Tokio runtime.
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .spawn_scoped(scope, || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .ok()?;
                runtime.block_on(bounded_log(
                    git_log(cwd, &remote, &branch),
                    &remote,
                    &branch,
                    deadline,
                ))
            })
            .ok()?
            .join()
            .ok()
            .flatten()
    })
}

async fn bounded_log(
    command: Command,
    remote: &str,
    branch: &str,
    deadline: Instant,
) -> Option<Stakes> {
    tokio::time::timeout_at(deadline.into(), dropped(command, remote, branch))
        .await
        .ok()
        .flatten()
}

fn force_push(command: &str) -> Option<(String, String)> {
    let words = shlex::split(command)?;
    let mut words = words.iter().map(String::as_str);
    if words.next()? != "git" || words.next()? != "push" {
        return None;
    }
    let mut force = false;
    let mut positional = Vec::new();
    let mut options = true;
    for word in words {
        match word {
            "--" if options => options = false,
            "-f" | "--force" | "--force-with-lease" if options => force = true,
            _ if ref_name(word) => positional.push(word),
            _ => return None,
        }
    }
    let [remote, branch] = positional.as_slice() else {
        return None;
    };
    force.then(|| (remote.to_string(), branch.to_string()))
}

fn ref_name(word: &str) -> bool {
    !word.is_empty()
        && !word.starts_with(['-', '.'])
        && !word.ends_with(['/', '.'])
        && !word.ends_with(".lock")
        && !word.contains("..")
        && !word.contains("//")
        && word
            .chars()
            .all(|ch| ch.is_alphanumeric() || "_-/.".contains(ch))
}

fn git_log(cwd: &Path, remote: &str, branch: &str) -> Command {
    let mut command = Command::new("git");
    command
        .args([
            "--no-pager",
            // An older Git must fail, not ignore an unknown no-fetch environment variable.
            "--no-lazy-fetch",
            "log",
            "--oneline",
            "--no-decorate",
            "--no-color",
            "--no-show-signature",
            "--end-of-options",
        ])
        .arg(format!(
            "refs/heads/{branch}..refs/remotes/{remote}/{branch}"
        ))
        .arg("--")
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    command
}

async fn dropped(command: Command, remote: &str, branch: &str) -> Option<Stakes> {
    let mut child = bingo_process::tree(command).ok()?;
    let mut lines = BufReader::new(child.take_stdout()?).lines();
    let mut count = 0;
    let mut items = Vec::new();
    while let Some(line) = lines.next_line().await.ok()? {
        count += 1;
        if items.len() < 5 {
            items.push(line);
        }
    }
    if !child.wait().await.ok()?.success() {
        return None;
    }
    let noun = if count == 1 { "commit" } else { "commits" };
    Some(Stakes {
        line: format!("drops {count} {noun} that only {remote}/{branch} has"),
        items,
    })
}

#[cfg(test)]
mod tests;
