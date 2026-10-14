//! Provider configuration is explicit in every spawned test process.
#![allow(dead_code)]
use std::ffi::OsStr;
use std::process::Command;

pub const PROVIDER_ENV: [&str; 4] = [
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "OPENAI_BASE_URL",
    "ANTHROPIC_BASE_URL",
];

pub fn at(binary: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(binary);
    for name in PROVIDER_ENV {
        command.env_remove(name);
    }
    command
}

pub fn bingo() -> Command {
    at(env!("CARGO_BIN_EXE_bingo"))
}

#[test]
fn provider_environment_is_removed_before_a_fixture_configures_its_endpoint() {
    let mut command = bingo();
    for name in PROVIDER_ENV {
        assert!(
            command
                .get_envs()
                .any(|(key, value)| key == name && value.is_none()),
            "inherited {name}"
        );
    }
    command.env("OPENAI_BASE_URL", "http://127.0.0.1:1234");
    assert!(
        command
            .get_envs()
            .any(|(key, value)| key == "OPENAI_BASE_URL"
                && value == Some(OsStr::new("http://127.0.0.1:1234")))
    );
}

pub fn pty() -> portable_pty::CommandBuilder {
    let mut command = portable_pty::CommandBuilder::new(env!("CARGO_BIN_EXE_bingo"));
    for name in PROVIDER_ENV {
        command.env_remove(name);
    }
    command
}
