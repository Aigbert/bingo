//! Files are opened only at the canonical path returned by scope.
use super::{
    scope::{self, Access, LIMIT, Roots},
    world::*,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use bingo_plugin_host::ops::Caller;
use serde_json::{Value, json};
use std::path::{Component, Path, PathBuf};
use tokio::io::AsyncReadExt;

pub struct Files {
    pub project: PathBuf,
    data: PathBuf,
}
impl Files {
    pub fn new(project: PathBuf, data: PathBuf) -> Self {
        Self { project, data }
    }
    fn data(&self, plugin: &str) -> Result<PathBuf, String> {
        let mut parts = Path::new(plugin).components();
        if !matches!(parts.next(), Some(Component::Normal(_))) || parts.next().is_some() {
            return Err("invalid plugin name".into());
        }
        Ok(self.data.join("plugins").join(plugin).join("data"))
    }
    pub fn path(&self, caller: &Caller, path: &Path, access: Access) -> Result<PathBuf, String> {
        scope::scope(
            path,
            Roots {
                project: &self.project,
                data: &self.data(&caller.plugin)?,
            },
            access,
        )
        .map_err(|e| e.to_string())
    }
    pub async fn call(&self, caller: &Caller, method: &str, input: Value) -> Result<Value, String> {
        match method {
            "read" => {
                let input: FileRead = parse(input)?;
                read(
                    &self.path(caller, &input.path, Access::Read)?,
                    input.encoding,
                )
                .await
                .map(Value::String)
            }
            "write" => self.write(caller, parse(input)?).await,
            "exists" => {
                let input: FilePath = parse(input)?;
                tokio::fs::try_exists(self.path(caller, &input.path, Access::Read)?)
                    .await
                    .map(Value::Bool)
                    .map_err(failed)
            }
            "stat" => {
                let input: FilePath = parse(input)?;
                let meta = tokio::fs::metadata(self.path(caller, &input.path, Access::Read)?)
                    .await
                    .map_err(failed)?;
                Ok(json!(FileStat {
                    size: meta.len(),
                    directory: meta.is_dir(),
                    file: meta.is_file()
                }))
            }
            "list" => self.list(caller, parse(input)?).await,
            "ancestors" => self.ancestors(caller, parse(input)?).await,
            _ => Err("unknown filesystem method".into()),
        }
    }
    async fn write(&self, caller: &Caller, input: FileWrite) -> Result<Value, String> {
        let path = match self.path(caller, &input.path, Access::Write) {
            Ok(path) => path,
            Err(_) => {
                let path = self.path(caller, &input.path, Access::GrantedWrite)?;
                bingo_plugin_host::grant::check(
                    &caller.host,
                    &caller.plugin,
                    "fs",
                    "write",
                    true,
                    &json!({"session":input.session}),
                )
                .await?;
                path
            }
        };
        scope::cap(
            input.body.len(),
            if matches!(input.encoding, Some(Encoding::Base64)) {
                LIMIT.div_ceil(3) * 4
            } else {
                LIMIT
            },
        )
        .map_err(failed)?;
        let bytes = match input.encoding {
            Some(Encoding::Base64) => STANDARD.decode(input.body).map_err(failed)?,
            _ => input.body.into_bytes(),
        };
        scope::cap(bytes.len(), LIMIT).map_err(failed)?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(failed)?;
        }
        tokio::fs::write(path, bytes).await.map_err(failed)?;
        Ok(Value::Null)
    }
    async fn list(&self, caller: &Caller, input: FileList) -> Result<Value, String> {
        let mut entries = tokio::fs::read_dir(self.path(caller, &input.path, Access::Read)?)
            .await
            .map_err(failed)?;
        let mut names = Vec::new();
        let mut bytes = 0;
        while names.len() < input.limit.unwrap_or(1000).min(LIMIT) {
            let Some(entry) = entries.next_entry().await.map_err(failed)? else {
                break;
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            bytes += name.len();
            scope::cap(bytes, LIMIT).map_err(failed)?;
            names.push(name);
        }
        names.sort();
        Ok(json!(names))
    }
    async fn ancestors(&self, caller: &Caller, input: Ancestors) -> Result<Value, String> {
        let mut name = Path::new(&input.name).components();
        if !matches!(name.next(), Some(Component::Normal(_))) || name.next().is_some() {
            return Err("ancestors requires one file name".into());
        }
        let session = caller
            .host
            .sessions(bingo_sdk::SessionFilter::default())
            .await
            .map_err(failed)?
            .into_iter()
            .find(|s| s.id == input.session)
            .ok_or("session is unavailable")?;
        let cwd = self.path(caller, Path::new(&session.cwd), Access::Read)?;
        let root = self.path(caller, &self.project, Access::Read)?;
        let mut matches = Vec::new();
        for directory in cwd.ancestors().take_while(|p| p.starts_with(&root)) {
            let path = self.path(caller, &directory.join(&input.name), Access::Read)?;
            if tokio::fs::try_exists(path).await.map_err(failed)? {
                matches.push(directory.to_path_buf());
            }
        }
        Ok(json!(matches))
    }
}
async fn read(path: &Path, encoding: Option<Encoding>) -> Result<String, String> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(failed)?
        .take((LIMIT + 1) as u64);
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).await.map_err(failed)?;
    scope::cap(bytes.len(), LIMIT).map_err(failed)?;
    match encoding {
        Some(Encoding::Base64) => Ok(STANDARD.encode(bytes)),
        _ => String::from_utf8(bytes)
            .map_err(|_| "file is not UTF-8; request base64 encoding".into()),
    }
}
pub(super) fn parse<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, String> {
    serde_json::from_value(value).map_err(failed)
}
fn failed(error: impl std::fmt::Display) -> String {
    error.to_string()
}
