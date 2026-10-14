//! Erasure and file resolution are shared by validation and execution.
use oxc_ast::{AstKind, ast::*};
use std::path::{Path, PathBuf};

pub(crate) fn runtime_specifier(kind: AstKind<'_>) -> Option<&str> {
    match kind {
        AstKind::ImportDeclaration(import) if !type_only(import) => {
            Some(import.source.value.as_str())
        }
        AstKind::ExportAllDeclaration(export)
            if export.export_kind == ImportOrExportKind::Value =>
        {
            Some(export.source.value.as_str())
        }
        AstKind::ExportFromDeclaration(export)
            if export.export_kind == ImportOrExportKind::Value
                && (export.specifiers.is_empty()
                    || export
                        .specifiers
                        .iter()
                        .any(|s| s.export_kind == ImportOrExportKind::Value)) =>
        {
            Some(export.source.value.as_str())
        }
        _ => None,
    }
}
fn type_only(import: &ImportDeclaration<'_>) -> bool {
    import.import_kind == ImportOrExportKind::Type
        || import.specifiers.as_ref().is_some_and(|specifiers| {
            !specifiers.is_empty()
                && specifiers.iter().all(|specifier| {
                    matches!(specifier, ImportDeclarationSpecifier::ImportSpecifier(specifier)
                    if specifier.import_kind == ImportOrExportKind::Type)
                })
        })
}

pub(crate) fn declaration(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            [".d.ts", ".d.mts", ".d.cts"]
                .iter()
                .any(|suffix| name.ends_with(suffix))
        })
}

pub(crate) fn module_path(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    let path = locate(root.join(relative));
    let path = path
        .canonicalize()
        .map_err(|error| format!("{}:1: {error}", relative.display()))?;
    if !path.starts_with(root) {
        return Err(format!(
            "{}:1: module escapes the mod root",
            relative.display()
        ));
    }
    if declaration(&path) || declaration(relative) {
        return Err(format!(
            "{}:1: a declaration file is not a runtime module",
            relative.display()
        ));
    }
    Ok(path)
}
fn locate(path: PathBuf) -> PathBuf {
    if path.is_file() {
        return path;
    }
    for extension in ["ts", "tsx", "d.ts", "js", "jsx"] {
        let candidate = if path.is_dir() {
            path.join(format!("index.{extension}"))
        } else if path.extension().is_none() {
            path.with_extension(extension)
        } else {
            continue;
        };
        if candidate.is_file() {
            return candidate;
        }
    }
    path
}
