//! The author's files and SDK run entirely outside this checkout.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::home;

use crate::isolated;

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
    isolated::bingo()
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
    let mut manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(plugin.join("plugin.json")).unwrap())
            .unwrap();
    manifest.as_object_mut().unwrap().remove("module");
    manifest["entry"] = serde_json::json!({"command":"node","args":["${PLUGIN_ROOT}/main.ts"]});
    std::fs::write(plugin.join("plugin.json"), manifest.to_string()).unwrap();
    let validated = success(&run(root.path(), "validate", &plugin));
    assert!(validated.contains("protocol 2") && validated.contains("tool.check"));
    println!("external TypeScript process validation ran");
}

#[test]
fn wrong_expectation_names_the_test_and_exits_one() {
    let root = tempfile::tempdir().unwrap();
    let plugin = fixture(root.path(), "ts-status");
    std::fs::write(plugin.join("hooks/register.test.ts"), "import { test, expect } from 'bingo/testing'; test('wrong expectation', () => expect(1).toBe(2));").unwrap();
    let output = run(root.path(), "test", &plugin);
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("wrong expectation")
            && stdout.contains("register.test.ts:")
            && stdout.contains("FAIL"),
        "{stdout}"
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
    assert!(
        !plugin.join(".bingo").exists(),
        "a process plugin is untyped"
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
fn tests_refuse_a_process_only_manifest() {
    let root = tempfile::tempdir().unwrap();
    let plugin = root.path().join("broken");
    std::fs::create_dir(&plugin).unwrap();
    std::fs::write(
        plugin.join("plugin.json"),
        r#"{"name":"broken","version":"1","entry":{"command":"must-not-run"}}"#,
    )
    .unwrap();
    let output = run(root.path(), "test", &plugin);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("module manifest"));
}

#[test]
fn module_acceptance_runs_outside_the_workspace() {
    let root = tempfile::tempdir().unwrap();
    let plugin = fixture(root.path(), "ts-status");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(plugin.join("plugin.json")).unwrap())
            .unwrap();
    manifest.as_object_mut().unwrap().remove("entry");
    manifest["module"] = serde_json::json!("hooks/register.ts");
    manifest["uses"] = serde_json::json!(["store", "ui"]);
    std::fs::write(plugin.join("plugin.json"), manifest.to_string()).unwrap();
    let tested = success(&run(root.path(), "test", &plugin));
    assert!(tested.starts_with("PASS ") && tested.contains("register.test.ts"));
    assert!(!plugin.join("must-not-run").exists());
}

#[test]
fn module_listing_names_the_runtime_and_its_disabled_reason() {
    let root = tempfile::tempdir().unwrap();
    let plugin = root.path().join(".bingo/plugins/hello");
    std::fs::create_dir_all(&plugin).unwrap();
    std::fs::write(
        plugin.join("plugin.json"),
        r#"{"name":"hello","version":"1","module":"main.ts"}"#,
    )
    .unwrap();
    std::fs::write(plugin.join("main.ts"), "export function register(on) {}").unwrap();
    let list = || {
        isolated::bingo()
            .args(["plugins", "list"])
            .current_dir(root.path())
            .envs(home::home_env(root.path()))
            .env_remove("BINGO_FAKE_SCRIPT")
            .output()
            .unwrap()
    };
    let first = success(&list());
    let row = first
        .lines()
        .find(|line| line.starts_with("hello\t"))
        .unwrap();
    assert!(row.ends_with("\tmodule"), "{row}");
    std::fs::write(
        root.path().join(".bingo/settings.toml"),
        "[enabledPlugins]\n\"bingo.mods\" = false\n",
    )
    .unwrap();
    let disabled = success(&list());
    let row = disabled
        .lines()
        .find(|line| line.starts_with("hello\t"))
        .unwrap();
    assert!(
        row.contains("\toff\t") && row.contains("module runtime is switched off"),
        "{row}"
    );
}

#[test]
fn the_cli_composes_the_module_runtime_for_an_explicit_directory() {
    let root = tempfile::tempdir().unwrap();
    let plugin = root.path().join("hello");
    std::fs::create_dir(&plugin).unwrap();
    std::fs::write(
        plugin.join("plugin.json"),
        r#"{"name":"hello","version":"1","module":"main.ts"}"#,
    )
    .unwrap();
    std::fs::write(plugin.join("main.ts"), "export function register(on) { on('tool.call', () => ({parts:[{type:'text',text:'module result'}],isError:false})); }").unwrap();
    let script = root.path().join("fake.json");
    std::fs::write(&script, r#"{"responses":[{"steps":[{"toolCall":{"name":"Read","input":{"file_path":"missing.txt"}}}]},{"steps":[{"text":"offline"}]}]}"#).unwrap();
    let output = isolated::bingo()
        .args([
            "--print",
            "--output-format",
            "json",
            "--provider",
            "fake",
            "--model",
            "fake-1",
            "--plugin-dir",
        ])
        .arg(&plugin)
        .arg("hello")
        .current_dir(root.path())
        .envs(home::home_env(root.path()))
        .env("BINGO_FAKE_SCRIPT", script)
        .output()
        .unwrap();
    let output = success(&output);
    let frames: Vec<bingo_sdk::Frame> = output
        .lines()
        .map(|line| serde_json::from_str(line).expect("stdout is NDJSON frames"))
        .collect();
    assert!(frames.iter().any(|frame| matches!(&frame.event, bingo_sdk::Event::ItemCompleted {item}
        if matches!(&item.body, bingo_sdk::ItemBody::ToolCall {output:Some(output), ..} if output.parts.iter().any(|part| part.as_text() == Some("module result"))))), "{output}");
}

#[test]
fn listing_includes_explicit_modules_without_starting_them() {
    let root = tempfile::tempdir().unwrap();
    let plugin = root.path().join("hello");
    std::fs::create_dir(&plugin).unwrap();
    std::fs::write(
        plugin.join("plugin.json"),
        r#"{"name":"hello","version":"1","module":"missing.ts"}"#,
    )
    .unwrap();
    let output = isolated::bingo()
        .arg("--plugin-dir")
        .arg(&plugin)
        .args(["plugins", "list"])
        .current_dir(root.path())
        .envs(home::home_env(root.path()))
        .env_remove("BINGO_FAKE_SCRIPT")
        .output()
        .unwrap();
    let text = success(&output);
    let row = text.lines().find(|line| line.starts_with("hello\t"));
    assert!(
        row.is_some_and(|row| row.contains("\trewrite") && row.ends_with("\tmodule")),
        "{text}"
    );
}

#[path = "support/reentry.rs"]
mod reentry;

#[path = "support/nouns.rs"]
mod nouns;

#[path = "support/startup.rs"]
mod startup;

fn world_output(root: &Path, plugin: &Path, url: Option<&str>) -> Vec<bingo_sdk::Frame> {
    let script = root.join("world-script.json");
    std::fs::write(&script,r#"{"responses":[{"steps":[{"toolCall":{"name":"Read","input":{"file_path":"world.txt"}}}]},{"steps":[{"text":"offline"}]}]}"#).unwrap();
    let output = isolated::bingo()
        .args([
            "--print",
            "--output-format",
            "json",
            "--provider",
            "fake",
            "--model",
            "fake-1",
            "--plugin-dir",
        ])
        .arg(plugin)
        .arg("world")
        .current_dir(root)
        .envs(home::home_env(root.join("user-home")))
        .env("BINGO_FAKE_SCRIPT", script)
        .env("BINGO_WORLD_PARITY", "environment")
        .env("BINGO_WORLD_URL", url.unwrap_or_default())
        .output()
        .unwrap();
    success(&output)
        .lines()
        .map(|line| serde_json::from_str(line).expect("NDJSON frames"))
        .collect()
}
#[test]
fn world_nouns_give_the_same_answer_in_process_and_under_node() {
    if !node_available() {
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let plugin = fixture(root.path(), "ts-status");
    std::fs::write(
        plugin.join("hooks/register.ts"),
        include_str!("../../bingo-mods/tests/mods/world/parity.ts"),
    )
    .unwrap();
    std::fs::write(root.path().join("world.txt"), "filesystem").unwrap();
    std::fs::create_dir_all(root.path().join("user-home/.bingo")).unwrap();
    std::fs::write(root.path().join("user-home/.bingo/settings.toml"),"[pluginTrust.ts-status]\ntier = \"rewrite\"\nuses = [\"fs\", \"http\", \"process\", \"env\", \"mcp\", \"agent\"]\n").unwrap();
    let mut answers = Vec::new();
    for node in [false, true] {
        let mut manifest = serde_json::json!({"name":"ts-status","version":"1","uses":["fs","http","process","env","mcp","agent"]});
        if node {
            manifest["entry"] =
                serde_json::json!({"command":"node","args":["${PLUGIN_ROOT}/main.ts"]});
        } else {
            manifest["module"] = serde_json::json!("hooks/register.ts");
        }
        std::fs::write(plugin.join("plugin.json"), manifest.to_string()).unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        // Nonblocking accept bounds a failed CLI without leaving a server thread behind.
        listener.set_nonblocking(true).unwrap();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stopped = stop.clone();
        let server = std::thread::spawn(move || {
            while !stopped.load(std::sync::atomic::Ordering::Relaxed) {
                match listener.accept() {
                    Ok((socket, _)) => {
                        respond_world_http(socket);
                        return;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(1))
                    }
                    Err(error) => panic!("{error}"),
                }
            }
        });
        let frames = world_output(root.path(), &plugin, Some(&url));
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        server.join().unwrap();
        let text = frames
            .iter()
            .find_map(|frame| match &frame.event {
                bingo_sdk::Event::ItemCompleted { item } => match &item.body {
                    bingo_sdk::ItemBody::ToolCall {
                        output: Some(output),
                        ..
                    } => output.parts.iter().find_map(|part| {
                        part.as_text()
                            .filter(|text| text.starts_with('{'))
                            .map(str::to_owned)
                    }),
                    _ => None,
                },
                _ => None,
            })
            .unwrap_or_else(|| panic!("world result (node={node}): {frames:#?}"));
        answers.push(serde_json::from_str::<serde_json::Value>(&text).unwrap());
    }
    assert_eq!(
        answers[0],
        serde_json::json!({"fs":"filesystem","env":"environment","process":"child","http":"http","agent":0,"mcp":true})
    );
    assert_eq!(answers[0], answers[1]);
}

fn respond_world_http(mut socket: std::net::TcpStream) {
    use std::io::{BufRead, BufReader, Write};
    // Accepted sockets can inherit the listener's nonblocking mode.
    socket.set_nonblocking(false).unwrap();
    socket
        .set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .unwrap();
    let mut request = BufReader::new(&mut socket);
    let mut line = String::new();
    loop {
        line.clear();
        assert_ne!(request.read_line(&mut line).unwrap(), 0);
        if line == "\r\n" {
            break;
        }
    }
    socket
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nhttp")
        .unwrap();
}

#[test]
fn the_models_spawn_agent_call_crosses_the_agent_noun_chain() {
    let root = tempfile::tempdir().unwrap();
    let plugin = root.path().join("agent-link");
    std::fs::create_dir(&plugin).unwrap();
    std::fs::write(
        plugin.join("plugin.json"),
        r#"{"name":"agent-link","version":"1","module":"main.ts"}"#,
    )
    .unwrap();
    std::fs::write(plugin.join("main.ts"),"export function register(on) { on('agent.spawn', () => ({parts:[{type:'text',text:'agent noun intercepted'}],isError:false})); }").unwrap();
    let script = root.path().join("script.json");
    std::fs::write(&script,r#"{"responses":[{"steps":[{"toolCall":{"name":"SpawnAgent","input":{"prompt":"must be intercepted"}}}]},{"steps":[{"text":"offline"}]}]}"#).unwrap();
    let output = isolated::bingo()
        .args([
            "--print",
            "--output-format",
            "json",
            "--provider",
            "fake",
            "--model",
            "fake-1",
            "--plugin-dir",
        ])
        .arg(&plugin)
        .arg("world")
        .current_dir(root.path())
        .envs(home::home_env(root.path()))
        .env("BINGO_FAKE_SCRIPT", script)
        .output()
        .unwrap();
    let frames: Vec<bingo_sdk::Frame> = success(&output)
        .lines()
        .map(|line| serde_json::from_str(line).expect("NDJSON frames"))
        .collect();
    assert!(frames.iter().any(|frame| matches!(&frame.event,bingo_sdk::Event::ItemCompleted {item} if matches!(&item.body,bingo_sdk::ItemBody::ToolCall {output:Some(output),..} if output.parts.iter().any(|part|part.as_text()==Some("agent noun intercepted"))))));
}

#[test]
fn module_tests_report_ndjson_and_use_the_real_host() {
    let home = tempfile::tempdir().unwrap();
    let plugin = home.path().join("kit");
    std::fs::create_dir(&plugin).unwrap();
    std::fs::write(
        plugin.join("plugin.json"),
        r#"{"name":"kit","version":"1","module":"main.ts","uses":["session"]}"#,
    )
    .unwrap();
    std::fs::write(plugin.join("main.ts"), "export const register = on => {};").unwrap();
    std::fs::write(plugin.join("main.test.ts"), "import {test,expect} from 'bingo/testing'; test('real host', async $ => expect(await $.session.id()).not.toBe(undefined)); test('wrong', () => expect(1).toBe(2));").unwrap();
    let output = isolated::bingo()
        .args(["plugins", "test"])
        .arg(&plugin)
        .arg("--json")
        .envs(home::home_env(home.path()))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let lines: Vec<serde_json::Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        lines.len(),
        2,
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(lines[0]["outcome"]["status"], "pass");
    assert_eq!(lines[1]["outcome"]["status"], "fail");
    assert!(
        lines[1]["outcome"]["at"]
            .as_str()
            .unwrap()
            .contains("main.test.ts:")
    );
}

#[test]
fn mounts_use_terminal_drawing_actions_and_invalidation() {
    let home = tempfile::tempdir().unwrap();
    let plugin = home.path().join("counter");
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mods/counter"),
        &plugin,
    );
    std::fs::write(
        plugin.join("counter.test.ts"),
        r#"import {test,expect} from 'bingo/testing';
      test('counter', async $ => {
        await $.command.run({name:'counter',args:{}});
        const pane = await $.ui.mount({pane:'counter'});
        expect(await pane.drawn()).toContain('Count: 0');
        await pane.press('i');
        expect(await pane.drawn()).toContain('Count: 1');
        await pane.advance(40);
        await pane.input('name','Ada');
        expect(JSON.stringify(await pane.tree())).toContain('Ada');
        expect(await pane.find('name')).toMatchObject({kind:'Input',key:'name'});
        await pane.resize(60,12);
        expect(await pane.drawn()).toContain('Count: 1');
      });"#,
    )
    .unwrap();
    let tested = success(&run(home.path(), "test", &plugin));
    assert!(tested.starts_with("PASS"), "{tested}");
}
#[test]
fn clock_and_environment_mocks_keep_grants_and_hook_budgets() {
    let home = tempfile::tempdir().unwrap();
    let plugin = home.path().join("clock");
    std::fs::create_dir(&plugin).unwrap();
    std::fs::write(
        plugin.join("plugin.json"),
        r#"{"name":"clock","version":"1","module":"main.ts","uses":["clock","env"]}"#,
    )
    .unwrap();
    std::fs::write(plugin.join("main.ts"), "export const register = on => {};").unwrap();
    std::fs::write(plugin.join("clock.test.ts"),r#"import {test,expect,mock} from 'bingo/testing';
      test('clock and env', async $ => {
        await mock.clock.set('2026-01-01T00:00:00Z');
        await mock.clock.advance(86400000);
        expect(await $.clock.now()).toBe(1767312000000);
        await mock.env({BINGO_TEST:'isolated'});
        expect(await $.env.get({name:'BINGO_TEST'})).toBe('isolated');
        expect(await $.env.get({name:'PATH'})).toBe(null);
        await expect($.http.fetch({url:'https://must-not-contact.invalid'})).rejects.toThrow('grant');
      });"#).unwrap();
    assert!(success(&run(home.path(), "test", &plugin)).starts_with("PASS"));
}

const LAID: [&str; 4] = [
    "bingo/index.d.ts",
    "bingo/events.ts",
    "bingo/testing.d.ts",
    "bingo-tools/index.d.ts",
];

fn laid_times(plugin: &Path) -> Vec<std::time::SystemTime> {
    LAID.iter()
        .map(|path| {
            std::fs::metadata(plugin.join(".bingo/types").join(path))
                .unwrap()
                .modified()
                .unwrap()
        })
        .collect()
}

#[test]
fn types_are_laid_on_request_and_keep_the_authors_tsconfig() {
    let home = tempfile::tempdir().unwrap();
    let plugin = fixture(home.path(), "ts-status");
    let authors = std::fs::read(plugin.join("tsconfig.json")).unwrap();
    let written = success(&run(home.path(), "types", &plugin));
    for path in LAID {
        let laid = Path::new(".bingo/types").join(path);
        assert!(
            written.lines().any(|line| Path::new(line).ends_with(&laid)),
            "{path} not listed: {written}"
        );
    }
    assert!(
        written.lines().all(|line| Path::new(line).is_file()),
        "{written}"
    );
    assert_eq!(
        std::fs::read(plugin.join("tsconfig.json")).unwrap(),
        authors
    );
    let first = laid_times(&plugin);
    assert_eq!(success(&run(home.path(), "types", &plugin)), "");
    success(&run(home.path(), "test", &plugin));
    assert_eq!(laid_times(&plugin), first);
    println!("type mtimes unchanged: {first:?}");
    let events = std::fs::read_to_string(plugin.join(".bingo/types/bingo/events.ts")).unwrap();
    assert!(
        events.contains("ToolCall") && events.contains("ClockNow") && events.contains("UiRender")
    );
    assert!(events.contains("agent.spawn"), "shipped services are typed");
}

#[test]
fn types_lay_the_reference_and_examples_that_validate_and_pass() {
    let home = tempfile::tempdir().unwrap();
    let plugin = fixture(home.path(), "band");
    let written = success(&run(home.path(), "types", &plugin));
    let reference = std::fs::read_to_string(plugin.join(".bingo/reference.md")).unwrap();
    assert!(reference.contains("### `tool.call`") && reference.contains("### `$.ui`"));
    for name in ["tool-call", "band", "pane", "ts-status"] {
        let example = plugin.join(".bingo/examples").join(name);
        assert!(
            written.contains(&format!("examples/{name}/plugin.json")),
            "{written}"
        );
        success(&run(home.path(), "validate", &example));
        let tested = success(&run(home.path(), "test", &example));
        assert!(
            tested.contains("PASS") && !tested.contains("FAIL"),
            "{name}: {tested}"
        );
    }
    let tested = success(&run(home.path(), "test", &plugin));
    assert!(
        !tested.contains(".bingo"),
        "the root runs only its own tests: {tested}"
    );
}

/// Each `$` noun's `@example` runs in the test runtime, so the text an author
/// copies from the types or the reference is code that works.
#[test]
fn every_noun_example_runs_in_the_test_runtime() {
    use bingo_mods::dollar::nouns::NOUNS;
    let home = tempfile::tempdir().unwrap();
    let plugin = home.path().join("nouns");
    std::fs::create_dir_all(&plugin).unwrap();
    let uses: Vec<&str> = NOUNS
        .iter()
        .filter(|noun| noun.runs && noun.name != "plugin")
        .map(|noun| noun.name)
        .collect();
    let manifest =
        serde_json::json!({"name": "nouns", "version": "1", "module": "register.ts", "uses": uses});
    std::fs::write(plugin.join("plugin.json"), manifest.to_string()).unwrap();
    std::fs::write(
        plugin.join("register.ts"),
        "export const register = on => {};\n",
    )
    .unwrap();
    let mut tests = String::from("import {test} from 'bingo/testing';\n");
    for noun in NOUNS {
        let run = if noun.runs { "test" } else { "test.skip" };
        tests.push_str(&format!(
            "{run}('$.{}', async $ => {{\n{}\n}});\n",
            noun.name, noun.example
        ));
    }
    std::fs::write(plugin.join("nouns.test.ts"), tests).unwrap();
    let tested = success(&run(home.path(), "test", &plugin));
    let passed = NOUNS.iter().filter(|noun| noun.runs).count();
    assert_eq!(tested.matches("PASS").count(), passed, "{tested}");
}

#[test]
fn validate_and_test_lay_types_but_never_into_a_process_plugin() {
    for verb in ["validate", "test"] {
        let home = tempfile::tempdir().unwrap();
        let plugin = fixture(home.path(), "band");
        success(&run(home.path(), verb, &plugin));
        assert!(
            plugin.join(".bingo/types/bingo/index.d.ts").is_file(),
            "{verb}"
        );
    }
    let home = tempfile::tempdir().unwrap();
    let plugin = fixture(home.path(), "wordcount");
    let refused = run(home.path(), "types", &plugin);
    assert_eq!(refused.status.code(), Some(1));
    assert!(refused.stdout.is_empty());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("module plugins"));
    assert!(!plugin.join(".bingo").exists() && !plugin.join("tsconfig.json").exists());
}

#[test]
fn component_examples_run_with_the_author_kit() {
    for name in ["tool-call", "band", "pane"] {
        let home = tempfile::tempdir().unwrap();
        let plugin = fixture(home.path(), name);
        assert!(
            success(&run(home.path(), "test", &plugin)).contains("PASS"),
            "{name}"
        );
    }
}
#[test]
fn sdk_helpers_run_without_a_mod_manifest() {
    let home = tempfile::tempdir().unwrap();
    let library = home.path().join("sdk");
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sdk/typescript"),
        &library,
    );
    let output = success(&run(home.path(), "test", &library));
    assert_eq!(
        output
            .lines()
            .filter(|line| line.starts_with("PASS "))
            .count(),
        4,
        "{output}"
    );
    assert!(output.contains("SKIP src/on.test.ts"), "{output}");
    assert!(!library.join("plugin.json").exists());
}

#[test]
fn a_directory_without_a_manifest_is_named_for_every_author_verb() {
    let home = tempfile::tempdir().unwrap();
    let empty = home.path().join("empty");
    std::fs::create_dir(&empty).unwrap();
    let absent = home.path().join("absent");
    // `test` also runs a library package, so its empty-directory line says so.
    for (verb, unlaid) in [
        ("types", "plugin.json: not found"),
        ("validate", "plugin.json: not found"),
        ("test", "a module manifest or a library package.json"),
    ] {
        for (dir, says) in [(&empty, unlaid), (&absent, "absent")] {
            let refused = run(home.path(), verb, dir);
            let stderr = String::from_utf8_lossy(&refused.stderr);
            assert_eq!(refused.status.code(), Some(1), "{verb}: {stderr}");
            assert!(refused.stdout.is_empty(), "{verb}");
            assert!(stderr.contains(says), "{verb}: {stderr}");
        }
    }
}
