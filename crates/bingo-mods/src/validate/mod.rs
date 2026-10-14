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
    SealedSite,
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
mod contract;
mod dollar;
mod helpers;
mod hooks;
mod keys;
mod props;
mod source;
mod uses;
pub fn validate(
    root: &Path,
    manifest: &bingo_plugin_host::manifest::Manifest,
) -> Result<Validation, String> {
    let bingo_plugin_host::manifest::Launch::Module(entry) = &manifest.launch else {
        return Err("module validation requires a module manifest".into());
    };
    let mut output = Validation {
        uses: manifest.uses.iter().cloned().collect(),
        ..Default::default()
    };
    let declared = contract::read(root, &manifest.name, &mut output)?;
    source::graph(root, entry, &manifest.name, declared.as_ref(), &mut output)?;
    uses::unused(&mut output);
    output
        .findings
        .sort_by(|a, b| (&a.file, a.line, a.column).cmp(&(&b.file, b.line, b.column)));
    Ok(output)
}
#[cfg(test)]
mod tests;
