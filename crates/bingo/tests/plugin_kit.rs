//! The author's files and SDK run entirely outside this checkout.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[path = "support/home.rs"]
mod home;

fn node_available() -> bool {
    let version = Command::new("node")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default();
    let numbers: Vec<u32> = version
        .trim()
        .trim_start_matches('v')
        .split('.')
        .filter_map(|part| part.parse().ok())
        .collect();
    let ready = matches!(numbers.as_slice(), [major, minor, ..] if *major >= 24 || (*major == 23 && *minor >= 6) || (*major == 22 && *minor >= 18));
    if !ready {
        assert!(
            std::env::var_os("BINGO_REQUIRE_NODE").is_none(),
            "CI requires a type-stripping Node"
        );
        eprintln!("SKIP plugin kit: Node >=22.18, >=23.6 or >=24 required; found {version:?}");
    }
    ready
}

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &to.join(entry.file_name()));
        } else {
            std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}

fn fixture(root: &Path, name: &str) -> PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let plugin = root.join("examples/plugins").join(name);
    copy(&workspace.join("examples/plugins").join(name), &plugin);
    if name == "ts-status" {
        copy(
            &workspace.join("sdk/typescript"),
            &root.join("sdk/typescript"),
        );
    }
    plugin
}

fn run(root: &Path, verb: &str, plugin: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bingo"))
        .args(["plugins", verb])
        .arg(plugin)
        .current_dir(root)
        .envs(home::home_env(root))
        .env_remove("BINGO_FAKE_SCRIPT")
        .output()
        .unwrap()
}

fn success(output: &Output) -> String {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).unwrap()
}

#[test]
fn typescript_acceptance_runs_outside_the_workspace() {
    if !node_available() {
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let plugin = fixture(root.path(), "ts-status");
    let validated = success(&run(root.path(), "validate", &plugin));
    assert!(validated.contains("protocol 2") && validated.contains("tool.check"));
    let tested = success(&run(root.path(), "test", &plugin));
    assert!(tested.starts_with("PASS ") && tested.contains("acceptance.json"));
    println!("external TypeScript acceptance ran (validate, state, reload, slot, denial)");
    assert!(!plugin.join("must-not-run").exists());
}

#[test]
fn wrong_expectation_names_the_step_and_exits_one() {
    if !node_available() {
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let plugin = fixture(root.path(), "ts-status");
    std::fs::write(plugin.join("scenarios/acceptance.json"), r#"{"model":{"responses":[]},"steps":[{"expect":{"slot":{"slot":"calls","contains":"wrong"}}}]}"#).unwrap();
    let output = run(root.path(), "test", &plugin);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(
        error.contains("step 1")
            && error.contains("expectation not met")
            && error.contains("slots:"),
        "{error}"
    );
}

#[test]
fn wordcount_validates_with_the_existing_protocol() {
    if Command::new("python3").arg("--version").output().is_err() {
        eprintln!("SKIP wordcount: python3 is not available");
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let plugin = fixture(root.path(), "wordcount");
    let text = success(&run(root.path(), "validate", &plugin));
    assert!(
        text.contains("protocol 2") && text.contains("tool:"),
        "{text}"
    );
}

#[test]
fn missing_entry_is_a_clean_validation_failure() {
    let root = tempfile::tempdir().unwrap();
    let plugin = root.path().join("broken");
    std::fs::create_dir(&plugin).unwrap();
    std::fs::write(
        plugin.join("plugin.json"),
        r#"{"name":"broken","version":"1"}"#,
    )
    .unwrap();
    let output = run(root.path(), "validate", &plugin);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("entry"));
}

#[test]
fn scenario_refuses_a_plugin_that_did_not_start() {
    let root = tempfile::tempdir().unwrap();
    let plugin = root.path().join("broken");
    std::fs::create_dir_all(plugin.join("scenarios")).unwrap();
    std::fs::write(
        plugin.join("plugin.json"),
        serde_json::json!({
            "name":"broken", "version":"1", "entry":{"command":plugin.join("missing-program")}
        })
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        plugin.join("scenarios/start.json"),
        r#"{"model":{"responses":[]},"steps":[]}"#,
    )
    .unwrap();
    let output = run(root.path(), "test", &plugin);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing-program"));
}
