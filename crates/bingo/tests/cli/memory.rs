use super::*;

fn invoke(home: &std::path::Path, command: &str, json: bool) -> Output {
    let fake = script(r#"{"responses":[]}"#);
    let mut cmd = bingo();
    cmd.envs(home_env(home))
        .env("BINGO_FAKE_SCRIPT", fake.path())
        .args(["--print", "--cwd"])
        .arg(home);
    if json {
        cmd.args(["--output-format", "json"]);
    }
    run(cmd.arg(command))
}
fn seed(scope: &std::path::Path) {
    std::fs::create_dir_all(scope).unwrap();
    std::fs::write(
        scope.join("rule.md"),
        "---\nname: rule\ntype: feedback\ndescription: Use fixtures\n---\nUse contracts.\n",
    )
    .unwrap();
    std::fs::write(scope.join("MEMORY.md"), "# Memory\n[rule](rule.md)\nkeep\n").unwrap();
}
#[test]
fn memory_forget_cli_is_ambiguous_until_scoped_and_emits_valid_ndjson() {
    let home = tempfile::tempdir().unwrap();
    let listing = invoke(home.path(), "/memory", false);
    assert_eq!(listing.status.code(), Some(0), "{}", stderr(&listing));
    let said = stdout(&listing);
    let project = std::path::PathBuf::from(said.trim().rsplit_once(" and ").unwrap().1);
    assert!(project.starts_with(home.path()));
    let user = home.path().join(".bingo/data/memory/user");
    seed(&user);
    seed(&project);
    let out = invoke(home.path(), "/memory forget rule", false);
    assert!(!out.status.success());
    assert!(stdout(&out).is_empty());
    assert!(
        stderr(&out).contains("INVALID_INPUT") && stderr(&out).contains("user and project"),
        "{}",
        stderr(&out)
    );
    assert!(user.join("rule.md").exists() && project.join("rule.md").exists());
    let out = invoke(home.path(), "/memory forget project rule", true);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let frames = frames_of(&out);
    assert!(!frames.is_empty());
    assert!(stdout(&out).contains("this session's prompt may keep it"));
    assert!(!project.join("rule.md").exists());
    assert!(user.join("rule.md").exists());
    assert_eq!(
        std::fs::read_to_string(project.join("MEMORY.md")).unwrap(),
        "# Memory\nkeep\n"
    );
}
#[test]
fn disabled_mod_keeps_both_tables_and_experience_forget_works() {
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join(".bingo");
    std::fs::create_dir(&config).unwrap();
    std::fs::write(
        config.join("settings.json"),
        r#"{"enabledPlugins":{"bingo.mod.memory":false},"update":{"check":false}}"#,
    )
    .unwrap();
    seed(&config.join("data/memory/user"));
    let library = bingo_experience::Library::new(&config);
    let entry = bingo_experience::entry::Entry {
        id: "fixture".into(),
        status: bingo_experience::entry::Status::Active,
        trigger: vec!["wire".into()],
        summary: "Use wire fixtures".into(),
        steps: vec!["Test the contract".into()],
        verify: None,
        created: jiff::Timestamp::UNIX_EPOCH,
        outcomes: vec![],
        notes: String::new(),
    };
    library.save(home.path(), &entry).unwrap();
    for (command, expected) in [
        ("/memory", "Use fixtures"),
        ("/experience", "Use wire fixtures"),
    ] {
        let out = invoke(home.path(), command, false);
        assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
        assert!(stdout(&out).contains(expected));
        assert!(!stdout(&out).contains("tokens carried"));
    }
    let out = invoke(home.path(), "/experience forget fixture", false);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(stdout(&out), "forgot experience fixture\n");
    assert!(!library.path(home.path(), "fixture").exists());
}
