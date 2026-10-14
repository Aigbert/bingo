use super::source::Source;
use oxc_ast::{
    AstKind,
    ast::{CallExpression, Expression},
};
use oxc_semantic::Semantic;

pub(super) fn calls(call: &CallExpression<'_>, semantic: &Semantic<'_>, source: &mut Source<'_>) {
    let Some(helper) = imported(&call.callee, semantic) else {
        return;
    };
    if matches!(helper.as_str(), "read" | "update") {
        source.output.calls.insert("state.get".into());
    }
    if helper == "update" {
        source.output.calls.insert("state.set".into());
    }
}
fn imported(expression: &Expression<'_>, semantic: &Semantic<'_>) -> Option<String> {
    let Expression::Identifier(id) = expression else {
        return None;
    };
    let symbol = semantic
        .scoping()
        .get_reference(id.reference_id.get()?)
        .symbol_id()?;
    let declaration = semantic.scoping().symbol_declaration(symbol);
    let AstKind::ImportSpecifier(specifier) = semantic.nodes().kind(declaration) else {
        return None;
    };
    let builtin = semantic.nodes().ancestor_kinds(declaration).any(
        |kind| matches!(kind,AstKind::ImportDeclaration(import) if import.source.value == "bingo"),
    );
    builtin.then(|| specifier.imported.name().to_string())
}
