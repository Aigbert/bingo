//! Walk each reachable source once, using the runtime's import boundary.
use super::*;
use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SourceType};
use std::path::PathBuf;

pub(super) struct Source<'a> {
    pub file: &'a str,
    pub plugin: &'a str,
    pub declared: Option<&'a BTreeSet<String>>,
    pub text: &'a str,
    pub output: &'a mut Validation,
}
impl Source<'_> {
    pub fn finding(
        &mut self,
        offset: u32,
        rule: Rule,
        severity: Severity,
        text: impl Into<String>,
    ) {
        let before = &self.text[..offset as usize];
        self.output.findings.push(Finding {
            file: self.file.into(),
            line: before.bytes().filter(|b| *b == b'\n').count() + 1,
            column: before
                .rsplit('\n')
                .next()
                .unwrap_or_default()
                .chars()
                .count()
                + 1,
            rule,
            severity,
            text: text.into(),
        });
    }
}
pub(super) fn graph(
    root: &Path,
    entry: &str,
    plugin: &str,
    declared: Option<&BTreeSet<String>>,
    output: &mut Validation,
) -> Result<(), String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut pending = vec![PathBuf::from(entry)];
    let mut seen = BTreeSet::new();
    while let Some(file) = pending.pop() {
        if !seen.insert(file.clone()) {
            continue;
        }
        let path = root.join(&file).canonicalize().map_err(|e| e.to_string())?;
        if !path.starts_with(&root) {
            return Err(format!("{}: module escapes the mod root", file.display()));
        }
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let name = file.to_string_lossy().replace('\\', "/");
        let mut source = Source {
            file: &name,
            plugin,
            declared,
            text: &text,
            output,
        };
        pending.extend(scan(&file, &mut source));
    }
    Ok(())
}
fn scan(file: &Path, source: &mut Source<'_>) -> Vec<PathBuf> {
    let allocator = Allocator::default();
    let kind = SourceType::from_path(file).unwrap_or_default();
    let parsed = Parser::new(&allocator, source.text, kind).parse();
    if let Some(error) = parsed.diagnostics.first() {
        source.finding(
            error.labels.first().map_or(0, |label| label.offset()),
            Rule::Import,
            Severity::Error,
            error.to_string(),
        );
        return Vec::new();
    }
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    let semantic = built.semantic;
    let mut imports = Vec::new();
    for node in semantic.nodes().iter() {
        let kind = node.kind();
        if let Some(specifier) = specifier(kind) {
            match crate::strip::resolve(file, specifier) {
                Ok(path) if specifier != "bingo" => imports.push(path),
                Ok(_) => (),
                Err(error) => {
                    source.finding(kind.span().start, Rule::Import, Severity::Error, error)
                }
            }
        }
        if matches!(kind, AstKind::ImportExpression(_)) {
            source.finding(
                kind.span().start,
                Rule::Import,
                Severity::Error,
                "dynamic import() is not available",
            );
        }
    }
    super::hooks::check(&semantic, source);
    super::dollar::check(&semantic, source);
    super::keys::check(&semantic, source);
    super::props::check(&semantic, source);
    imports
}
fn specifier(kind: AstKind<'_>) -> Option<&str> {
    match kind {
        AstKind::ImportDeclaration(import) => Some(import.source.value.as_str()),
        AstKind::ExportAllDeclaration(export) => Some(export.source.value.as_str()),
        AstKind::ExportFromDeclaration(export) => Some(export.source.value.as_str()),
        _ => None,
    }
}
