//! `bingo plugins list|enable|disable` and what a switch does to the next run
//! (ADR-0057). The answer is the one thing on stdout; every diagnostic is on
//! stderr and a refusal is one `[error]` line and a non-zero exit.

use super::*;

fn user_settings(home: &std::path::Path) -> std::path::PathBuf {
    home.join(".bingo/settings.toml")
}

fn settings_json(home: &std::path::Path) -> serde_json::Value {
    super::settings::read(&user_settings(home))
}

fn plugins(home: &std::path::Path) -> Command {
    let mut cmd = bingo();
    cmd.envs(home_env(home))
        .arg("--cwd")
        .arg(home)
        .arg("plugins");
    cmd
}

/// One row of a listing, by the name it starts with.
fn row<'a>(listing: &'a str, name: &str) -> Vec<&'a str> {
    let prefix = format!("{name}\t");
    listing
        .lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("no {name} row in {listing}"))
        .split('\t')
        .collect()
}

/// A `--print` run over the fake provider, with the frames on stdout so a
/// tool's answer can be read back.
fn scripted(home: &std::path::Path, script: &tempfile::NamedTempFile, prompt: &str) -> Output {
    run(bingo()
        .env("BINGO_FAKE_SCRIPT", script.path())
        .envs(home_env(home))
        .args(["--print", "--output-format", "json", "--cwd"])
        .arg(home)
        .arg(prompt))
}

/// The one response a script needs when the command answers by itself.
const UNUSED: &str = r#"{"responses":[{"steps":[{"text":"unused"}]}]}"#;

const FETCH: &str = r#"{"responses":[
    {"steps":[{"toolCall":{"name":"WebFetch","input":{"url":"https://example.com"}}}]},
    {"steps":[{"text":"I could not fetch it."}]}
]}"#;

/// A switch is written to the user layer, shown in the listing, and taken at
/// the next start: the model is not offered a tool whose plugin is off.
#[test]
fn a_tool_plugin_switched_off_is_written_listed_and_gone_from_the_model_s_tools() {
    let home = tempfile::tempdir().unwrap();
    let fetch = script(FETCH);
    let before = scripted(home.path(), &fetch, "fetch it");
    assert!(
        !stdout(&before).contains("tool not found"),
        "the tool is there until it is switched off: {}",
        stdout(&before)
    );

    let off = run(plugins(home.path()).args(["disable", "bingo.tools.web"]));
    assert_eq!(off.status.code(), Some(0), "stderr: {}", stderr(&off));
    assert_eq!(stdout(&off), "bingo.tools.web is off at the next start.\n");
    assert_eq!(stderr(&off), "");
    assert_eq!(
        settings_json(home.path())["enabledPlugins"],
        serde_json::json!({ "bingo.tools.web": false })
    );
    assert!(
        !home.path().join(".bingo/settings.local.json").exists(),
        "a project file is never written"
    );

    let listed = run(plugins(home.path()).arg("list"));
    assert_eq!(listed.status.code(), Some(0), "stderr: {}", stderr(&listed));
    assert_eq!(stderr(&listed), "");
    let said = stdout(&listed);
    assert_eq!(row(&said, "bingo.tools.web")[2], "off");
    assert_eq!(
        row(&said, "bingo.tools.web")[3],
        "switched off in the settings"
    );
    assert_eq!(
        row(&said, "bingo.tools.fs")[2],
        "on",
        "and its neighbours are"
    );

    let again = script(FETCH);
    let after = scripted(home.path(), &again, "fetch it");
    assert_eq!(after.status.code(), Some(0), "stderr: {}", stderr(&after));
    assert!(
        stdout(&after).contains("tool not found: WebFetch"),
        "the model reached for a tool this run does not have: {}",
        stdout(&after)
    );

    let unused = script(UNUSED);
    let table = run(bingo()
        .env("BINGO_FAKE_SCRIPT", unused.path())
        .envs(home_env(home.path()))
        .args(["--print", "--cwd"])
        .arg(home.path())
        .arg("/plugins"));
    assert_eq!(table.status.code(), Some(0), "stderr: {}", stderr(&table));
    let drawn = stdout(&table);
    assert!(drawn.contains("bingo.tools.web"), "{drawn}");
    assert!(drawn.contains("switched off in the settings"), "{drawn}");

    let on = run(plugins(home.path()).args(["enable", "bingo.tools.web"]));
    assert_eq!(stdout(&on), "bingo.tools.web is on at the next start.\n");
    assert_eq!(
        settings_json(home.path())["enabledPlugins"],
        serde_json::json!({ "bingo.tools.web": true }),
        "a switch flips rather than being taken out"
    );
}

/// The two switches nobody may write: one the binary cannot run without, and
/// a name no listing knows (ADR-0057 §3).
#[test]
fn a_needed_plugin_and_a_name_nobody_lists_are_refused_with_the_reason() {
    let home = tempfile::tempdir().unwrap();

    let refused = run(plugins(home.path()).args(["disable", "bingo.store.jsonl"]));
    assert_eq!(refused.status.code(), Some(1));
    assert_eq!(stdout(&refused), "");
    let said = stderr(&refused);
    assert!(said.contains("code=INVALID_INPUT"), "{said}");
    assert!(said.contains("ignores its switch"), "{said}");
    assert!(
        !user_settings(home.path()).exists(),
        "a refusal writes nothing"
    );

    let unknown = run(plugins(home.path()).args(["disable", "bingo.tools.wb"]));
    assert_eq!(unknown.status.code(), Some(1));
    assert_eq!(stdout(&unknown), "");
    assert!(
        stderr(&unknown).contains("bingo plugins list"),
        "a person with the wrong spelling is told where to look: {}",
        stderr(&unknown)
    );
    assert!(!user_settings(home.path()).exists());
}

/// The example this repository ships, installed the way a person installs one.
fn install_wordcount(home: &std::path::Path) {
    let example = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/plugins/wordcount");
    let plugin = home.join(".bingo/plugins/wordcount");
    std::fs::create_dir_all(&plugin).unwrap();
    for file in ["plugin.json", "main.py"] {
        std::fs::copy(example.join(file), plugin.join(file)).unwrap();
    }
}

const COUNT: &str = r#"{"responses":[
    {"steps":[{"toolCall":{"name":"plugin__wordcount__count","input":{"path":"notes.txt"}}}]},
    {"steps":[{"text":"Counted."}]}
]}"#;

/// An external plugin is switched by the same key and listed in the same
/// table, and switched off its process is never spawned — so its tool never
/// reaches the model (ADR-0057 §4, §5). Skipped without `python3`, as the
/// bridge's own black-box suite is.
#[test]
fn an_external_plugin_is_listed_switched_and_never_spawned() {
    if !python::python3() {
        eprintln!("skipped: no python3");
        return;
    }
    let home = tempfile::tempdir().unwrap();
    install_wordcount(home.path());
    std::fs::write(home.path().join("notes.txt"), "alpha beta\n").unwrap();

    let listed = run(plugins(home.path()).arg("list"));
    let said = stdout(&listed);
    assert_eq!(
        row(&said, "wordcount")[1],
        "0.1.0",
        "its manifest's version"
    );
    assert_eq!(row(&said, "wordcount")[2], "on");

    let off = run(plugins(home.path()).args(["disable", "wordcount"]));
    assert_eq!(off.status.code(), Some(0), "stderr: {}", stderr(&off));
    assert_eq!(stdout(&off), "wordcount is off at the next start.\n");
    let said = stdout(&run(plugins(home.path()).arg("list")));
    assert_eq!(row(&said, "wordcount")[2], "off");
    assert_eq!(row(&said, "wordcount")[3], "switched off in the settings");

    let count = script(COUNT);
    let out = scripted(home.path(), &count, "count the words in notes.txt");
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(
        stdout(&out).contains("tool not found: plugin__wordcount__count"),
        "the process was never spawned, so it contributed nothing: {}",
        stdout(&out)
    );

    let unused = script(UNUSED);
    let table = run(bingo()
        .env("BINGO_FAKE_SCRIPT", unused.path())
        .envs(home_env(home.path()))
        .args(["--print", "--cwd"])
        .arg(home.path())
        .arg("/modules"));
    assert_eq!(table.status.code(), Some(0), "stderr: {}", stderr(&table));
    let drawn = stdout(&table);
    assert!(
        drawn.contains("wordcount") && drawn.contains("plugin-rpc"),
        "the bridge's own plugins are in the kernel's table, named for it: {drawn}"
    );
}

#[test]
fn a_missing_used_service_is_named_without_disabling_its_consumer() {
    let home = tempfile::tempdir().unwrap();
    let off = run(plugins(home.path()).args(["disable", "bingo.mcp"]));
    assert_eq!(off.status.code(), Some(0), "{}", stderr(&off));
    let listed = run(plugins(home.path()).arg("list"));
    assert_eq!(listed.status.code(), Some(0), "{}", stderr(&listed));
    assert_eq!(stderr(&listed), "");
    let said = stdout(&listed);
    let consumer = row(&said, "bingo.acp");
    assert_eq!(consumer[2], "on");
    assert!(
        consumer[3].contains("missing used services: mcp.servers"),
        "{said}"
    );
}

#[test]
fn an_external_plugins_tier_is_listed_from_user_settings_only() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    install_wordcount(home.path());
    let listed = run(plugins(home.path()).arg("list"));
    assert_eq!(listed.status.code(), Some(0));
    assert_eq!(row(&stdout(&listed), "wordcount")[4], "tighten");
    std::fs::write(
        user_settings(home.path()),
        "[pluginTrust.wordcount]\ntier = 'rewrite'\nuses = ['fs']\n",
    )
    .unwrap();
    std::fs::create_dir_all(project.path().join(".bingo")).unwrap();
    std::fs::write(
        project.path().join(".bingo/settings.toml"),
        "[pluginTrust.wordcount]\ntier = 'full'\n",
    )
    .unwrap();
    let listed = run(bingo()
        .envs(home_env(home.path()))
        .arg("--cwd")
        .arg(project.path())
        .args(["plugins", "list"]));
    assert_eq!(listed.status.code(), Some(0), "{}", stderr(&listed));
    assert_eq!(row(&stdout(&listed), "wordcount")[4], "rewrite");
    assert!(stderr(&listed).contains("grants belong in user settings"));
    assert!(!stdout(&listed).contains("ignored"));
}

#[test]
fn the_demo_is_opt_in_and_legacy_settings_report_the_new_switch() {
    let home = tempfile::tempdir().unwrap();
    let absent = run(plugins(home.path()).arg("list"));
    assert_eq!(absent.status.code(), Some(0), "{}", stderr(&absent));
    assert!(!stdout(&absent).contains("bingo.demo.ui"));
    std::fs::create_dir_all(home.path().join(".bingo")).unwrap();
    std::fs::write(user_settings(home.path()), "demoUi = true\n").unwrap();
    let legacy = scripted(home.path(), &script(UNUSED), "/board");
    assert_eq!(legacy.status.code(), Some(0), "{}", stderr(&legacy));
    assert!(
        stderr(&legacy).contains("enabledPlugins"),
        "{}",
        stderr(&legacy)
    );
    std::fs::write(
        user_settings(home.path()),
        "[enabledPlugins]\n\"bingo.demo.ui\" = true\n",
    )
    .unwrap();
    let switched = run(plugins(home.path()).arg("list"));
    assert_eq!(switched.status.code(), Some(0), "{}", stderr(&switched));
    assert_eq!(row(&stdout(&switched), "bingo.demo.ui")[2], "on");
}

#[test]
fn surface_values_match_the_help_snapshot() {
    let out = run(bingo().arg("--help"));
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(stderr(&out), "");
    let help = stdout(&out);
    let values: Vec<_> = help
        .lines()
        .filter_map(|line| line.find("[possible values:").map(|at| &line[at..]))
        .collect();
    assert_eq!(
        values,
        [
            "[possible values: text, json, stream-json]",
            "[possible values: text, stream-json]",
            "[possible values: stdio]",
        ]
    );
}

#[cfg(unix)]
#[test]
fn repository_processes_wait_for_a_byte_bound_personal_approval() {
    let home = tempfile::tempdir().unwrap();
    let repo = tempfile::tempdir().unwrap();
    let dir = repo.path().join(".bingo/plugins/repo-code");
    std::fs::create_dir_all(&dir).unwrap();
    let marker = repo.path().join("ran");
    let manifest = serde_json::json!({"name":"repo-code", "version":"1", "entry":{
        "command":"sh", "args":["-c", "printf ran > \"$MARKER\""], "env":{"MARKER":marker}}});
    std::fs::write(dir.join("plugin.json"), manifest.to_string()).unwrap();
    let root = bingo_core::approval::root_key(repo.path()).unwrap();
    let digest = bingo_core::approval::digest_dir(&dir).unwrap();
    // Neither the approval record nor an explicit-directory alias may authorize itself.
    std::fs::write(
        repo.path().join(".bingo/settings.json"),
        serde_json::json!({
            "approved":{root:{"plugin:repo-code":digest}}, "pluginDirs":[dir]
        })
        .to_string(),
    )
    .unwrap();
    let before = repository_print(home.path(), repo.path(), "/plugins");
    assert_eq!(before.status.code(), Some(0), "{}", stderr(&before));
    assert!(!marker.exists(), "repository process ran before the yes");
    assert!(
        stderr(&before).contains("PROJECT_WAITING"),
        "{}",
        stderr(&before)
    );
    assert!(stderr(&before).contains("bingo plugins approve repo-code"));
    assert!(stderr(&before).contains("APPROVED_IGNORED"));
    let approved = run(bingo()
        .envs(home_env(home.path()))
        .arg("--cwd")
        .arg(repo.path())
        .args(["plugins", "approve", "repo-code"]));
    assert_eq!(approved.status.code(), Some(0), "{}", stderr(&approved));
    assert!(!marker.exists(), "headless approval must not start a host");
    let after = repository_print(home.path(), repo.path(), "/plugins");
    assert_eq!(after.status.code(), Some(0), "{}", stderr(&after));
    assert!(
        marker.exists(),
        "approved process did not run: {}",
        stderr(&after)
    );
    std::fs::remove_file(&marker).unwrap();
    std::fs::write(dir.join("extra.txt"), "new code").unwrap();
    let changed = repository_print(home.path(), repo.path(), "/plugins");
    assert!(!marker.exists(), "changed process reused a stale yes");
    assert!(
        stderr(&changed).contains("changed since your yes"),
        "{}",
        stderr(&changed)
    );
}

#[cfg(unix)]
fn repository_print(home: &std::path::Path, repo: &std::path::Path, prompt: &str) -> Output {
    let fake = script(UNUSED);
    let output = run(bingo()
        .envs(home_env(home))
        .env("BINGO_FAKE_SCRIPT", fake.path())
        .arg("--cwd")
        .arg(repo)
        .args(["--print", "--output-format", "stream-json", prompt]));
    for line in stdout(&output).lines() {
        serde_json::from_str::<serde_json::Value>(line).expect("stdout is pure NDJSON");
    }
    output
}

#[cfg(unix)]
#[test]
fn repository_stdio_servers_cannot_dial_through_enable_or_reconnect() {
    let home = tempfile::tempdir().unwrap();
    let repo = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(repo.path().join(".bingo")).unwrap();
    let marker = repo.path().join("mcp-ran");
    std::fs::write(
        repo.path().join(".bingo/settings.json"),
        serde_json::json!({"mcpServers":{"repo-server":{
            "command":"/bin/sh", "args":["-c", "printf ran > \"$1\"", "sh", marker]
        }}})
        .to_string(),
    )
    .unwrap();
    for prompt in [
        "/mcp",
        "/mcp enable repo-server",
        "/mcp reconnect repo-server",
    ] {
        let output = repository_print(home.path(), repo.path(), prompt);
        assert!(!marker.exists(), "MCP ran before approval: {prompt}");
        assert!(
            stderr(&output).contains("PROJECT_WAITING"),
            "{}",
            stderr(&output)
        );
    }
    let approved = run(bingo()
        .envs(home_env(home.path()))
        .arg("--cwd")
        .arg(repo.path())
        .args(["mcp", "approve", "repo-server"]));
    assert_eq!(approved.status.code(), Some(0), "{}", stderr(&approved));
    assert!(!marker.exists());
    assert!(settings_json(home.path())["approved"].is_object());
}

#[cfg(unix)]
#[test]
fn escaped_plugin_entry_is_refused_even_with_a_matching_saved_digest() {
    let home = tempfile::tempdir().unwrap();
    let repo = tempfile::tempdir().unwrap();
    let dir = repo.path().join(".bingo/plugins/escape");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(repo.path().join("outside.sh"), "touch ran").unwrap();
    std::fs::write(
        dir.join("plugin.json"),
        serde_json::json!({
            "name":"escape", "version":"1", "entry":{
                "command":"sh", "args":["${PLUGIN_ROOT}/../../../outside.sh"]
            }
        })
        .to_string(),
    )
    .unwrap();
    let root = bingo_core::approval::root_key(repo.path()).unwrap();
    let digest = bingo_core::approval::digest_dir(&dir).unwrap();
    std::fs::create_dir_all(home.path().join(".bingo")).unwrap();
    std::fs::write(
        home.path().join(".bingo/settings.json"),
        serde_json::json!({
            "approved":{root:{"plugin:escape":digest}}
        })
        .to_string(),
    )
    .unwrap();
    let approved = run(bingo()
        .envs(home_env(home.path()))
        .arg("--cwd")
        .arg(repo.path())
        .args(["plugins", "approve", "escape"]));
    assert!(!approved.status.success());
    assert!(stderr(&approved).contains("plugin:escape"));
    let output = repository_print(home.path(), repo.path(), "/plugins");
    assert_eq!(output.status.code(), Some(0));
    assert!(!dir.join("ran").exists());
    assert!(
        format!("{}{}", stdout(&output), stderr(&output)).contains("escapes the plugin root"),
        "stdout={} stderr={}",
        stdout(&output),
        stderr(&output)
    );
}

#[cfg(unix)]
#[test]
fn repository_acp_adapter_waits_before_model_catalogue_probe() {
    let home = tempfile::tempdir().unwrap();
    let repo = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(repo.path().join(".bingo")).unwrap();
    let marker = repo.path().join("acp-ran");
    std::fs::write(
        repo.path().join(".bingo/settings.json"),
        serde_json::json!({
            "acp":{"adapters":{"repository":{
                "command":"sh", "args":["-c", "printf ran > \"$MARKER\""], "env":{"MARKER":marker}
            }}}
        })
        .to_string(),
    )
    .unwrap();
    let listed = repository_print(home.path(), repo.path(), "/model");
    assert_eq!(listed.status.code(), Some(0), "{}", stderr(&listed));
    assert!(!marker.exists());
    assert!(stderr(&listed).contains("/acp approve repository"));
    let approved = repository_print(home.path(), repo.path(), "/acp approve repository");
    assert_eq!(approved.status.code(), Some(0), "{}", stderr(&approved));
    assert!(!marker.exists());
    let root = bingo_core::approval::root_key(repo.path()).unwrap();
    assert!(settings_json(home.path())["approved"][root]["acp.adapters:repository"].is_string());
}

#[test]
fn a_session_switch_keeps_ndjson_stdout_pure_and_reports_timing() {
    for (name, source, expected) in [
        (
            "drawing",
            "export function register(on) { on('ui.render', () => h('Text',{},'hello')); }",
            "drawing is off now.",
        ),
        (
            "compose",
            "export function register(on) { on('prompt.compose', ($, e, next) => next(e)); }",
            "compose is off at next start (prompt.compose).",
        ),
    ] {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().join(".bingo/plugins").join(name);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("plugin.json"),
            serde_json::json!({"name":name,"version":"1","module":"main.ts","uses":[]}).to_string(),
        )
        .unwrap();
        std::fs::write(root.join("main.ts"), source).unwrap();
        let fake = script(UNUSED);
        let output = run(bingo()
            .envs(home_env(home.path()))
            .env("BINGO_FAKE_SCRIPT", fake.path())
            .arg("--cwd")
            .arg(home.path())
            .args(["--print", "--output-format", "json"])
            .arg(format!("/plugins disable {name}")));
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        let text = stdout(&output);
        assert!(text.contains(expected), "{text}");
        for line in text.lines() {
            serde_json::from_str::<serde_json::Value>(line).expect("pure NDJSON");
        }
        assert_eq!(settings_json(home.path())["enabledPlugins"][name], false);
    }
}
