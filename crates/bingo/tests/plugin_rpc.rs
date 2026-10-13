//! Black-box: the wordcount example plugin, driven through the real binary.
//!
//! The exit criterion of ADR-0015 §7 — a third party writes one Tool and one
//! Command in a language that is not Rust, and both run. Nothing here knows
//! anything about the bridge's insides: a directory with a `plugin.json` in
//! it, a prompt, and what came out.
//!
//! Every test skips where `python3` is absent, so a machine without it says so
//! rather than failing — except on CI, where a missing interpreter is a
//! failure rather than a suite that passes without running (`support/python`).

// An integration test is not `cfg(test)`; the test-only lint relief is spelled out.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use bingo_sdk::{Event, Frame, InteractionKind, ItemBody};

#[path = "support/home.rs"]
mod home;
use home::home_env;

#[path = "support/python.rs"]
mod python;
use python::python3;

/// The example this repository ships, which is what a person would copy.
fn example() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/wordcount")
}

/// A home with the example installed the way a person installs one, and a
/// project with something to count in it.
fn installed() -> (tempfile::TempDir, tempfile::TempDir) {
    let home = tempfile::tempdir().unwrap();
    let plugin = home.path().join(".bingo/plugins/wordcount");
    std::fs::create_dir_all(&plugin).unwrap();
    for file in ["plugin.json", "main.py"] {
        std::fs::copy(example().join(file), plugin.join(file)).unwrap();
    }
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("notes.txt"),
        "alpha beta gamma\ndelta epsilon\n",
    )
    .unwrap();
    (home, project)
}

fn script(dir: &Path, json: &str) -> PathBuf {
    let path = dir.join("script.json");
    std::fs::write(&path, json).unwrap();
    path
}

fn bingo(home: &Path, project: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_bingo"));
    command
        .envs(home_env(home))
        .env_remove("BINGO_FAKE_SCRIPT")
        .args(["--print", "--cwd"])
        .arg(project)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

fn run(command: &mut Command) -> Output {
    command.output().expect("the binary runs")
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).expect("utf-8 stdout")
}

fn stderr(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).expect("utf-8 stderr")
}

fn frames(out: &Output) -> Vec<Frame> {
    stdout(out)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap_or_else(|e| panic!("{e}: {line}")))
        .collect()
}

/// The script that makes the model call the plugin's tool.
const COUNT: &str = r#"{"responses":[
    {"steps":[{"toolCall":{"name":"plugin__wordcount__count","input":{"path":"notes.txt"}}}]},
    {"steps":[{"text":"Counted."}]}
]}"#;

#[test]
fn the_plugin_s_tool_reaches_the_model_named_for_the_plugin_and_untrusted() {
    if !python3() {
        eprintln!("skipped: no python3");
        return;
    }
    let (home, project) = installed();
    let script = script(home.path(), COUNT);
    let out = run(bingo(home.path(), project.path())
        .env("BINGO_FAKE_SCRIPT", &script)
        .args(["--output-format", "json"])
        .arg("count the words in notes.txt"));
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));

    let frames = frames(&out);
    let asked = frames.iter().find_map(|f| match &f.event {
        Event::InteractionOpened { interaction } => match &interaction.kind {
            InteractionKind::Permission { tool, .. } => Some(tool.clone()),
            _ => None,
        },
        _ => None,
    });
    assert_eq!(
        asked.as_deref(),
        Some("plugin__wordcount__count"),
        "the gate asked about the plugin's tool, by the name the card shows"
    );
    let denied = frames.iter().any(|f| {
        matches!(
            &f.event,
            Event::InteractionResolved {
                answer: bingo_sdk::Answer::Deny { .. },
                ..
            }
        )
    });
    assert!(denied, "and nobody was at the keyboard to allow it");
}

#[test]
fn the_plugin_s_tool_counts_the_file_the_session_is_working_in() {
    if !python3() {
        eprintln!("skipped: no python3");
        return;
    }
    let (home, project) = installed();
    let script = script(home.path(), COUNT);
    let out = run(bingo(home.path(), project.path())
        .env("BINGO_FAKE_SCRIPT", &script)
        .args(["--output-format", "json", "--dangerously-skip-permissions"])
        .arg("count the words in notes.txt"));
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));

    let output = frames(&out).into_iter().find_map(|f| match f.event {
        Event::ItemCompleted { item } => match item.body {
            ItemBody::ToolCall {
                name,
                output: Some(output),
                ..
            } if name == "plugin__wordcount__count" => Some(output),
            _ => None,
        },
        _ => None,
    });
    let output = output.expect("the plugin's tool ran and answered");
    assert!(!output.is_error, "{output:?}");
    assert_eq!(
        output.parts[0].as_text(),
        Some("5 words, 2 lines, 31 characters"),
        "the call carried the session's working directory, so the file was found"
    );
    let table = output.display.expect("a plugin ships a View of its own");
    assert!(table.fold().contains("notes.txt"), "{}", table.fold());
}

#[test]
fn the_plugin_s_command_answers_with_its_view() {
    if !python3() {
        eprintln!("skipped: no python3");
        return;
    }
    let (home, project) = installed();
    // The command answers by itself, but a session still needs a provider
    // (f251b1f: without a script there is no fake one). The response is
    // never consumed.
    let script = script(
        home.path(),
        r#"{"responses":[{"steps":[{"text":"unused"}]}]}"#,
    );
    let out = run(bingo(home.path(), project.path())
        .env("BINGO_FAKE_SCRIPT", &script)
        .arg("/wordcount notes.txt"));
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let said = stdout(&out);
    assert!(said.contains("notes.txt"), "{said}");
    assert!(said.contains('5') && said.contains("31"), "{said}");
}

/// With no `plugins/` directory anywhere the bridge is inert: the binary
/// behaves exactly as it did before this plugin existed.
#[test]
fn a_host_with_no_plugins_installed_runs_as_it_always_did() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let script = script(
        home.path(),
        r#"{"responses":[{"steps":[{"text":"Hello from the fake provider."}]}]}"#,
    );
    let out = run(bingo(home.path(), project.path())
        .env("BINGO_FAKE_SCRIPT", &script)
        .arg("hello"));
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "Hello from the fake provider.\n");
    assert_eq!(stderr(&out), "");
}

mod support;

const SIDE_PLUGIN: &str = r#"
import json, sys
held = {}
def send(value):
    print(json.dumps(dict(jsonrpc='2.0', **value)), flush=True)
def request(id, method, params):
    send(dict(id=id, method=method, params=params))
for line in sys.stdin:
    message = json.loads(line)
    method = message.get('method')
    params = message.get('params', {})
    if method == 'initialize':
        send(dict(id=message['id'], result=dict(protocol=2, name='side', version='1',
            capabilities=params['capabilities'], links=[dict(point='prompt.compose')])) )
    elif method == 'intercept/prompt.compose':
        held = message
        site = params['site']
        if sys.argv[1] == 'complete':
            request('side', 'host/model/complete', dict(session=site['session'], turn=site['turn'], request=params['input']))
        else:
            request('entered', 'host/extend', dict(session=site['session'], key='entered', value=params['chain']))
    elif method == 'intercept/cancel':
        request('cancelled', 'host/extend', dict(session=held['params']['site']['session'], key='cancelled', value=params['chain']))
    elif message.get('id') == 'side':
        send(dict(id=held['id'], result=dict(next=held['params']['input'])))
"#;

fn side_home(mode: &str) -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join(".bingo/plugins/side");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.py"), SIDE_PLUGIN).unwrap();
    std::fs::write(root.join("plugin.json"), serde_json::json!({
        "name":"side", "version":"1", "entry":{"command":"python3","args":["${PLUGIN_ROOT}/main.py",mode]}
    }).to_string()).unwrap();
    std::fs::write(
        home.path().join(".bingo/settings.json"),
        r#"{"pluginTrust":{"side":{"tier":"rewrite","uses":["model"]}}}"#,
    )
    .unwrap();
    home
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stub_completion_is_in_the_turns_terminal_bill() {
    use bingo_sdk::{HostApi, Input, IntentId, OpenOptions, Origin, Usage};
    if !python3() {
        return;
    }
    let mut server = support::Server::spawn_at(
        side_home("complete"),
        r#"{"responses":[{"steps":[{"text":"side response"}]},{"steps":[{"text":"main response"}]}]}"#,
        &[],
    );
    let kernel = support::ready(&mut server).await;
    let mut attachment = kernel
        .open(
            support::create(server.cwd()),
            support::who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    attachment.handle.submit(
        IntentId::mint(),
        Input::text("hello", Origin::surface("test")),
    );
    let frames = support::until_completed(&mut attachment).await;
    let mut sum = Usage::default();
    let mut charged = false;
    for frame in &frames {
        match &frame.event {
            Event::UsageCharged {
                turn,
                plugin,
                usage,
            } => {
                assert_eq!(plugin, "side");
                assert!(turn.is_some());
                assert!(usage.output_tokens > 0);
                charged = true;
                sum.add(*usage);
            }
            Event::TurnUsage { usage, .. } => sum.add(*usage),
            Event::TurnCompleted { usage, .. } => assert_eq!(usage, &sum),
            _ => {}
        }
    }
    assert!(charged, "the side call was billed: {frames:?}");
    kernel.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn interrupting_a_remote_first_phase_cancels_its_chain() {
    use bingo_sdk::{HostApi, Input, IntentId, InterruptScope, OpenOptions, Origin, TurnStatus};
    use futures::StreamExt;
    if !python3() {
        return;
    }
    let mut server = support::Server::spawn_at(
        side_home("hold"),
        r#"{"responses":[{"steps":[{"text":"unused"}]}]}"#,
        &[],
    );
    let kernel = support::ready(&mut server).await;
    let mut attachment = kernel
        .open(
            support::create(server.cwd()),
            support::who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    attachment.handle.submit(
        IntentId::mint(),
        Input::text("hello", Origin::surface("test")),
    );
    let mut chain = None;
    tokio::time::timeout(support::LIMIT, async {
        while let Some(frame) = attachment.events.next().await {
            if let Event::Extension { kind, payload, .. } = frame.event
                && kind == "entered"
            {
                chain = Some(payload);
                break;
            }
        }
    })
    .await
    .unwrap();
    attachment
        .handle
        .interrupt(IntentId::mint(), InterruptScope::Head);
    let mut cancelled = false;
    let mut interrupted = false;
    tokio::time::timeout(support::LIMIT, async {
        while !cancelled || !interrupted {
            match attachment.events.next().await.unwrap().event {
                Event::Extension { kind, payload, .. } if kind == "cancelled" => {
                    assert_eq!(Some(payload), chain);
                    cancelled = true;
                }
                Event::TurnCompleted { status, .. } => {
                    assert!(matches!(status, TurnStatus::Interrupted { .. }));
                    interrupted = true;
                }
                _ => {}
            }
        }
    })
    .await
    .unwrap();
    kernel.shutdown().await.unwrap();
}

#[test]
fn an_explicit_plugin_directory_is_loaded_by_the_cli() {
    if !python3() {
        return;
    }
    let (home, project) = installed();
    let external = tempfile::tempdir().unwrap();
    let root = external.path().join("wordcount");
    std::fs::rename(home.path().join(".bingo/plugins/wordcount"), &root).unwrap();
    let script = script(home.path(), COUNT);
    let out = run(bingo(home.path(), project.path())
        .env("BINGO_FAKE_SCRIPT", &script)
        .arg("--plugin-dir")
        .arg(&root)
        .args(["--output-format", "json", "--dangerously-skip-permissions"])
        .arg("count the words in notes.txt"));
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(frames(&out).iter().any(|frame| matches!(&frame.event,
        Event::ItemCompleted { item } if matches!(&item.body,
            ItemBody::ToolCall { name, output: Some(_), .. } if name == "plugin__wordcount__count"))));
}
