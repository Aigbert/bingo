//! Parity artifacts stay outside the checkout and preserve their source.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::path::Path;
use std::process::Command;

#[path = "support/home.rs"]
mod home;

fn workspace() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[cfg(unix)]
#[test]
fn port_layout_changes_only_the_package_specifier() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("originals");
    std::fs::create_dir(&source).unwrap();
    let module = "import { fixture } from 'claude-code';\nexport const marker = 146;\n";
    let state = "declare module 'claude-code' { interface Fixture {} }\n";
    for file in ["band.tsx", "pane.tsx", "tool-call.ts"] {
        std::fs::write(source.join(file), module).unwrap();
    }
    for file in ["band-state.d.ts", "pane-state.d.ts"] {
        std::fs::write(source.join(file), state).unwrap();
    }
    let binary = probe(temp.path());
    let ports = temp.path().join("ports");
    let output = Command::new("bash")
        .arg(workspace().join("scripts/port_examples.sh"))
        .args([&source, &ports])
        .env("BINGO_BIN", binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for (name, file) in [
        ("turn-band", "band.tsx"),
        ("tool-calls", "pane.tsx"),
        ("tool-call", "tool-call.ts"),
    ] {
        assert_eq!(std::fs::read_to_string(source.join(file)).unwrap(), module);
        assert_eq!(
            std::fs::read_to_string(ports.join(name).join("hooks").join(file)).unwrap(),
            module.replace("'claude-code'", "'bingo'")
        );
        assert!(ports.join(name).join("register.parity.test.ts").is_file());
        assert!(!ports.join(name).join("register.test.ts").exists());
    }
    assert_eq!(
        std::fs::read_to_string(ports.join("turn-band/types/index.d.ts")).unwrap(),
        state.replace("'claude-code'", "'bingo'")
    );
    let invocations = std::fs::read_to_string(ports.join(".home/invocations")).unwrap();
    assert_eq!(
        invocations
            .lines()
            .filter(|line| *line == "plugins validate")
            .count(),
        3
    );
    assert_eq!(
        invocations
            .lines()
            .filter(|line| *line == "plugins test")
            .count(),
        3
    );
}

#[cfg(unix)]
fn probe(root: &Path) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let binary = root.join("probe");
    std::fs::write(
        &binary,
        "#!/bin/sh\nprintf '%s\\n' \"$1 $2\" >> \"$HOME/invocations\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
    binary
}

fn copy_example(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() == ".bingo" {
            continue;
        }
        if entry.file_type().unwrap().is_dir() {
            copy_example(&entry.path(), &destination.join(entry.file_name()));
        } else {
            std::fs::copy(entry.path(), destination.join(entry.file_name())).unwrap();
        }
    }
}

#[test]
fn committed_examples_validate_and_test_in_an_isolated_home() {
    for name in ["tool-call", "band", "pane"] {
        let temp = tempfile::tempdir().unwrap();
        let plugin = temp.path().join(name);
        copy_example(&workspace().join("examples/plugins").join(name), &plugin);
        for verb in ["validate", "test"] {
            let output = Command::new(env!("CARGO_BIN_EXE_bingo"))
                .args(["plugins", verb])
                .arg(&plugin)
                .current_dir(temp.path())
                .envs(home::home_env(temp.path()))
                .env_remove("BINGO_FAKE_SCRIPT")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{name} {verb}: {}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            println!("{name} {verb}: exit 0");
        }
    }
}

#[test]
fn tool_call_agrees_under_quickjs_and_node() {
    let available = Command::new("node")
        .arg("--version")
        .output()
        .is_ok_and(|out| out.status.success());
    if !available {
        assert!(
            std::env::var_os("BINGO_REQUIRE_NODE").is_none(),
            "Linux CI requires Node"
        );
        eprintln!("SKIP mods parity: Node is unavailable");
        return;
    }
    assert_eq!(tool_evidence(false), tool_evidence(true));
}

fn tool_evidence(node: bool) -> Vec<serde_json::Value> {
    use serde_json::json;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join(".bingo")).unwrap();
    std::fs::write(
        root.join(".bingo/settings.json"),
        json!({"update":{"check":false},"pluginTrust":{"tool-call":{"tier":"rewrite"}}})
            .to_string(),
    )
    .unwrap();
    let plugin = root.join("tool-call");
    copy_example(&workspace().join("examples/plugins/tool-call"), &plugin);
    if node {
        install_node(root, &plugin);
    }
    std::fs::write(root.join(".env"), "old").unwrap();
    let script = root.join("fake.json");
    std::fs::write(&script, json!({"responses":[
        {"steps":[{"toolCall":{"name":"Edit","input":{"file_path":".env","old_string":"old","new_string":"new"}}}]},
        {"steps":[{"toolCall":{"name":"Bash","input":{"command":"  exit 7  "}}}]},
        {"steps":[{"toolCall":{"name":"Bash","input":{"command":"  echo parity  "}}}]},
        {"steps":[{"text":"Parity complete."}]}
    ]}).to_string()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_bingo"))
        .args([
            "--print",
            "--output-format",
            "json",
            "--provider",
            "fake",
            "--model",
            "fake-1",
            "--dangerously-skip-permissions",
            "--plugin-dir",
        ])
        .arg(&plugin)
        .arg("Exercise the guard and shell")
        .current_dir(root)
        .envs(home::home_env(root))
        .env("BINGO_FAKE_SCRIPT", script)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    let snapshot = fold_frames(&text);
    let calls: Vec<_> = snapshot
        .transcript
        .items()
        .iter()
        .filter_map(|item| {
            if let bingo_sdk::ItemBody::ToolCall { name, output, .. } = &item.body {
                Some(json!({"name":name,"status":item.status,"output":output}))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(calls.len(), 3, "{text}");
    assert_eq!(
        std::fs::read_to_string(root.join(".env")).unwrap(),
        "old",
        "node={node}: {text}"
    );
    assert!(calls[0].to_string().contains("protected"));
    assert!(calls[0].to_string().contains("tool-call"));
    assert_eq!(calls[0]["status"], "failed");
    assert_eq!(calls[1]["output"]["isError"], true);
    assert!(
        calls[2].to_string().contains("$ echo parity"),
        "node={node}: {calls:?}\n{text}"
    );
    assert!(text.contains("failed:"));
    assert!(
        !serde_json::to_string(&snapshot.signals)
            .unwrap()
            .contains("failed:")
    );
    println!(
        "{}: deny left file unchanged; shell trimmed; failure status cleared",
        if node { "Node" } else { "QuickJS" }
    );
    calls
}

fn install_node(root: &Path, plugin: &Path) {
    use serde_json::{Value, json};
    std::fs::write(
        root.join("package.json"),
        r#"{"name":"bingo","type":"module","exports":"./sdk/src/index.ts"}"#,
    )
    .unwrap();
    copy_example(&workspace().join("sdk/typescript"), &root.join("sdk"));
    let mut manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(plugin.join("plugin.json")).unwrap())
            .unwrap();
    manifest.as_object_mut().unwrap().remove("module");
    manifest["entry"] = json!({"command":"node","args":["${PLUGIN_ROOT}/main.ts"]});
    std::fs::write(plugin.join("plugin.json"), manifest.to_string()).unwrap();
    std::fs::write(
        plugin.join("main.ts"),
        "import {plugin} from 'bingo'; await plugin(new URL('./register.ts',import.meta.url));",
    )
    .unwrap();
}

fn fold_frames(text: &str) -> bingo_sdk::SessionState {
    let frames: Vec<bingo_sdk::Frame> = text
        .lines()
        .map(|line| serde_json::from_str(line).expect("stdout is NDJSON frames"))
        .collect();
    let summary = frames
        .iter()
        .find_map(|frame| match &frame.event {
            bingo_sdk::Event::SessionUpdated { summary } => Some(summary.clone()),
            _ => None,
        })
        .expect("session summary");
    let mut snapshot = bingo_sdk::SessionState::new(summary);
    for frame in &frames {
        snapshot.apply(frame);
    }
    snapshot
}
