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
    let mut command = at(env!("CARGO_BIN_EXE_bingo"));
    command.env("BINGO_FAKE_WAKE_FLOOR_MS", "1000");
    command
}

pub fn pty() -> portable_pty::CommandBuilder {
    let mut command = portable_pty::CommandBuilder::new(env!("CARGO_BIN_EXE_bingo"));
    for name in PROVIDER_ENV {
        command.env_remove(name);
    }
    command
}
