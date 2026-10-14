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
    use std::os::unix::fs::PermissionsExt;
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
    let binary = temp.path().join("probe");
    std::fs::write(
        &binary,
        "#!/bin/sh\nprintf '%s\\n' \"$1 $2\" >> \"$HOME/invocations\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
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
        ("band", "band.tsx"),
        ("pane", "pane.tsx"),
        ("tool-call", "tool-call.ts"),
    ] {
        assert_eq!(std::fs::read_to_string(source.join(file)).unwrap(), module);
        assert_eq!(
            std::fs::read_to_string(ports.join(name).join("hooks").join(file)).unwrap(),
            module.replace("'claude-code'", "'bingo'")
        );
        assert!(ports.join(name).join("register.test.ts").is_file());
    }
    assert_eq!(
        std::fs::read_to_string(ports.join("band/types/index.d.ts")).unwrap(),
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
