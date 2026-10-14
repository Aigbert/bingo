//! Static findings are stable, source-located author diagnostics.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Rule {
    OnLiteral,
    DollarStatic,
    StateKey,
    EnvName,
    Uses,
    StateContract,
    PropSubset,
    Import,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub rule: Rule,
    pub severity: Severity,
    pub text: String,
}
#[derive(Debug, Default, Serialize)]
pub struct Validation {
    pub findings: Vec<Finding>,
    pub events: BTreeSet<String>,
    pub calls: BTreeSet<String>,
    pub keys: BTreeSet<String>,
    pub uses: BTreeSet<String>,
}
impl Validation {
    pub fn valid(&self) -> bool {
        !self
            .findings
            .iter()
            .any(|finding| finding.severity == Severity::Error)
    }
}
pub fn validate(
    _root: &Path,
    _manifest: &bingo_plugin_host::manifest::Manifest,
) -> Result<Validation, String> {
    Ok(Validation::default())
}
#[cfg(test)]
mod tests;
