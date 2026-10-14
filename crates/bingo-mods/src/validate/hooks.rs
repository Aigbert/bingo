use super::{source::Source, *};
use oxc_ast::{AstKind, ast::*};
use oxc_semantic::Semantic;

pub(super) fn check(semantic: &Semantic<'_>, source: &mut Source<'_>) {
    for node in semantic.nodes().iter() {
        let AstKind::CallExpression(call) = node.kind() else {
            continue;
        };
        if !matches!(&call.callee, Expression::Identifier(id) if id.name == "on") {
            continue;
        }
        match call
            .arguments
            .first()
            .and_then(|arg| pattern(arg, semantic))
        {
            Some(pattern) => {
                source.output.events.insert(pattern);
            }
            None => source.finding(
                call.span.start,
                Rule::OnLiteral,
                Severity::Error,
                "on pattern must be a literal or a const string",
            ),
        }
        if call.arguments.len() > 2 && !call.arguments.get(1).is_some_and(matcher) {
            source.finding(
                call.span.start,
                Rule::OnLiteral,
                Severity::Error,
                "on matcher must be literal",
            );
        }
    }
}
fn pattern(arg: &Argument<'_>, semantic: &Semantic<'_>) -> Option<String> {
    if let Argument::StringLiteral(value) = arg {
        return Some(value.value.to_string());
    }
    let Argument::Identifier(id) = arg else {
        return None;
    };
    let symbol = semantic
        .scoping()
        .get_reference(id.reference_id.get()?)
        .symbol_id()?;
    for node in semantic.nodes().iter() {
        let AstKind::VariableDeclarator(var) = node.kind() else {
            continue;
        };
        if !matches!(semantic.nodes().parent_kind(node.id()), AstKind::VariableDeclaration(declaration) if declaration.kind == VariableDeclarationKind::Const)
        {
            continue;
        }
        if let BindingPattern::BindingIdentifier(binding) = &var.id
            && binding.symbol_id.get() == Some(symbol)
            && let Some(Expression::StringLiteral(value)) = &var.init
        {
            return Some(value.value.to_string());
        }
    }
    None
}
fn matcher(arg: &Argument<'_>) -> bool {
    arg.as_expression().is_some_and(literal)
}
fn literal(value: &Expression<'_>) -> bool {
    match value {
        Expression::StringLiteral(_) | Expression::NumericLiteral(_) | Expression::BooleanLiteral(_) | Expression::NullLiteral(_) | Expression::RegExpLiteral(_) => true,
        Expression::ArrayExpression(array) => array.elements.iter().all(|item| item.as_expression().is_some_and(literal)),
        Expression::ObjectExpression(object) => object.properties.iter().all(|property| matches!(property, ObjectPropertyKind::ObjectProperty(p) if !p.computed && !p.shorthand && literal(&p.value))),
        _ => false,
    }
}
