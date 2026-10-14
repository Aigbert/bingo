use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::path::Path;

pub fn imports_testing(path: &Path) -> Result<bool, String> {
    let source = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let allocator = Allocator::default();
    let kind = SourceType::from_path(path).map_err(|e| e.to_string())?;
    let parsed = Parser::new(&allocator, &source, kind).parse();
    if let Some(error) = parsed.diagnostics.first() {
        return Err(format!("{}: {error}", path.display()));
    }
    Ok(parsed.program.body.iter().any(|statement| {
        matches!(statement,Statement::ImportDeclaration(import) if import.source.value == "bingo/testing")
    }))
}
