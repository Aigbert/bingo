//! The world operations' transport-independent contracts.
use bingo_sdk::SessionId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

macro_rules! contract {
    ($($name:ident { $($(#[doc = $doc:literal])* $field:ident : $ty:ty),* $(,)? })*) => {$ (
        #[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
        #[serde(rename_all = "camelCase")]
        pub struct $name { $($(#[doc = $doc])* pub $field: $ty),* }
    )*};
}
contract! {
    FilePath {
        session: SessionId,
        /// Relative to the session's cwd, or absolute under a granted root.
        path: PathBuf,
    }
    FileRead {
        session: SessionId,
        /// Relative to the session's cwd, or absolute under a granted root.
        path: PathBuf,
        /// `utf8` (the default) or `base64`.
        encoding: Option<Encoding>,
    }
    FileWrite {
        session: SessionId,
        /// Relative to the session's cwd, or absolute under a granted root.
        path: PathBuf,
        /// The new contents.
        body: String,
        /// How `body` is encoded: `utf8` (the default) or `base64`.
        encoding: Option<Encoding>,
    }
    FileList {
        session: SessionId,
        /// The directory to list.
        path: PathBuf,
        /// At most this many entries.
        limit: Option<usize>,
    }
    FileStat {
        /// Size in bytes.
        size: u64,
        /// Whether it is a directory.
        directory: bool,
        /// Whether it is a regular file.
        file: bool,
    }
    Ancestors {
        session: SessionId,
        /// One file or directory name, without a separator.
        name: String,
    }
    Fetch {
        session: SessionId,
        /// The URL to fetch.
        url: String,
        /// `GET` unless given.
        method: Option<String>,
        /// Request headers.
        headers: Option<BTreeMap<String, String>>,
        /// The request body, as text.
        body: Option<String>,
    }
    Fetched {
        /// The HTTP status.
        status: u16,
        /// Response headers.
        headers: BTreeMap<String, String>,
        /// The body, as text.
        body: String,
        /// Whether the body was cut at the host's limit.
        truncated: bool,
        /// A note from the host about the fetch, if any.
        notice: Option<String>,
    }
    ProcessInput {
        session: SessionId,
        /// The program and its arguments; no shell is involved.
        argv: Vec<String>,
        /// Working directory; the session's cwd unless given.
        cwd: Option<PathBuf>,
        /// Kill the process after this many milliseconds.
        timeout_ms: Option<u64>,
    }
    ProcessId {
        session: SessionId,
        /// The id `spawn` answered.
        id: String,
    }
    ProcessWrite {
        session: SessionId,
        /// The id `spawn` answered.
        id: String,
        /// Text for stdin; `null` closes it.
        body: Option<String>,
    }
    ProcessOutput {
        /// The exit code, or `null` when killed by a signal.
        code: Option<i32>,
        /// Everything written to stdout.
        stdout: String,
        /// Everything written to stderr.
        stderr: String,
    }
    EnvGet {
        session: SessionId,
        /// The variable's name.
        name: String,
    }
    EnvSet {
        session: SessionId,
        /// The variable's name.
        name: String,
        /// Its value.
        value: String,
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Encoding {
    Utf8,
    Base64,
}
