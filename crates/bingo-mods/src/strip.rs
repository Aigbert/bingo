//! Compile a module without giving its source another way out of the root.
use std::path::{Component, Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SourceType};
use oxc_transformer::{JsxRuntime, TransformOptions, Transformer};

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct StripError(pub String);

pub fn strip(path: &Path, source: &str) -> Result<String, StripError> {
    compile(path, source, false)
}

pub(crate) fn compile(path: &Path, source: &str, testing: bool) -> Result<String, StripError> {
    let allocator = Allocator::default();
    let source_type =
        SourceType::from_path(path).map_err(|why| at(path, source, 0, &why.to_string()))?;
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if let Some(error) = parsed.diagnostics.first() {
        return Err(at(
            path,
            source,
            error
                .labels
                .first()
                .map_or(0, |label| label.offset() as usize),
            &error.to_string(),
        ));
    }
    let mut program = parsed.program;
    let semantics = SemanticBuilder::new()
        .with_build_nodes(true)
        .with_check_syntax_error(true)
        .with_enum_eval(true)
        .build(&program);
    if let Some(error) = semantics.diagnostics.first() {
        return Err(at(
            path,
            source,
            error
                .labels
                .first()
                .map_or(0, |label| label.offset() as usize),
            &error.to_string(),
        ));
    }
    for node in semantics.semantic.nodes().iter() {
        check(path, source, node.kind(), testing)?;
    }
    let mut options = TransformOptions::default();
    options.jsx.runtime = JsxRuntime::Classic;
    options.jsx.pragma = Some("h".into());
    options.jsx.pragma_frag = Some("Fragment".into());
    let transformed = Transformer::new(&allocator, path, &options)
        .build_with_scoping(semantics.semantic.into_scoping(), &mut program);
    if let Some(error) = transformed.diagnostics.first() {
        return Err(at(
            path,
            source,
            error
                .labels
                .first()
                .map_or(0, |label| label.offset() as usize),
            &error.to_string(),
        ));
    }
    Ok(Codegen::new().build(&program).code)
}

fn check(path: &Path, source: &str, kind: AstKind<'_>, testing: bool) -> Result<(), StripError> {
    let denied = |why: &str| at(path, source, kind.span().start as usize, why);
    let specifier = match kind {
        AstKind::ImportDeclaration(import) => Some(import.source.value.as_str()),
        AstKind::ExportAllDeclaration(export) => Some(export.source.value.as_str()),
        AstKind::ExportFromDeclaration(export) => Some(export.source.value.as_str()),
        AstKind::ImportExpression(_) => return Err(denied("dynamic import() is not available")),
        AstKind::IdentifierReference(id) if id.name == "require" => {
            return Err(denied("require is not available"));
        }
        _ => None,
    };
    if let Some(specifier) = specifier {
        if testing && specifier == "bingo/testing" {
            return Ok(());
        }
        resolve(path, specifier).map_err(|why| denied(&why))?;
    }
    Ok(())
}

pub fn resolve(base: &Path, specifier: &str) -> Result<PathBuf, String> {
    if specifier == "bingo" {
        return Ok(PathBuf::from("bingo"));
    }
    if !(specifier.starts_with("./") || specifier.starts_with("../"))
        || specifier.contains(['\\', '\0', ':'])
    {
        return Err(format!(
            "import {specifier:?} must be relative inside the mod root or bingo"
        ));
    }
    let mut path = PathBuf::new();
    let parent = base.parent().unwrap_or(Path::new(""));
    for part in parent.join(specifier).components() {
        match part {
            Component::Normal(name) => path.push(name),
            Component::CurDir => (),
            Component::ParentDir if path.pop() => (),
            _ => return Err(format!("import {specifier:?} escapes the mod root")),
        }
    }
    Ok(path)
}

fn at(path: &Path, source: &str, offset: usize, why: &str) -> StripError {
    let line = source
        .as_bytes()
        .iter()
        .take(offset)
        .filter(|byte| **byte == b'\n')
        .count()
        + 1;
    StripError(format!("{}:{line}: {why}", path.display()))
}
