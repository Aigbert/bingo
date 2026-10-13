//! The world operations' transport-independent contracts.
use bingo_sdk::SessionId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

macro_rules! contract {
    ($($name:ident { $($field:ident : $ty:ty),* $(,)? })*) => {$ (
        #[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
        #[serde(rename_all = "camelCase")]
        pub struct $name { $(pub $field: $ty),* }
    )*};
}
contract! {
    FilePath { session: SessionId, path: PathBuf }
    FileRead { session: SessionId, path: PathBuf, encoding: Option<Encoding> }
    FileWrite { session: SessionId, path: PathBuf, body: String, encoding: Option<Encoding> }
    FileList { session: SessionId, path: PathBuf, limit: Option<usize> }
    FileStat { size: u64, directory: bool, file: bool }
    Ancestors { session: SessionId, name: String }
    Fetch { session: SessionId, url: String, method: Option<String>, headers: Option<BTreeMap<String, String>>, body: Option<String> }
    Fetched { status: u16, headers: BTreeMap<String, String>, body: String, truncated: bool, notice: Option<String> }
    ProcessInput { session: SessionId, argv: Vec<String>, cwd: Option<PathBuf>, timeout_ms: Option<u64> }
    ProcessId { session: SessionId, id: String }
    ProcessWrite { session: SessionId, id: String, body: Option<String> }
    ProcessOutput { code: Option<i32>, stdout: String, stderr: String }
    EnvGet { session: SessionId, name: String }
    EnvSet { session: SessionId, name: String, value: String }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Encoding {
    Utf8,
    Base64,
}
