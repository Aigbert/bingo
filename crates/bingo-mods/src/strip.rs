use std::path::Path;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct StripError(pub String);

pub fn strip(path: &Path, _source: &str) -> Result<String, StripError> {
    Err(StripError(format!("{}:1: module compiler unavailable", path.display())))
}
