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
        if call.arguments.len() > 2
            && !call.arguments.get(1).is_some_and(|arg| {
                arg.as_expression()
                    .is_some_and(|value| literal(value, semantic, 0))
            })
        {
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
fn constant<'a>(
    id: &IdentifierReference<'a>,
    semantic: &Semantic<'a>,
) -> Option<&'a Expression<'a>> {
    let scoping = semantic.scoping();
    let symbol = scoping.get_reference(id.reference_id.get()?).symbol_id()?;
    if scoping.symbol_scope_id(symbol) != scoping.root_scope_id() {
        return None;
    }
    for node in semantic.nodes().iter() {
        let AstKind::VariableDeclarator(var) = node.kind() else {
            continue;
        };
        if !matches!(semantic.nodes().parent_kind(node.id()), AstKind::VariableDeclaration(declaration) if declaration.kind == VariableDeclarationKind::Const)
        {
            continue;
        }
        if matches!(&var.id, BindingPattern::BindingIdentifier(binding) if binding.symbol_id.get() == Some(symbol))
        {
            let value = var.init.as_ref()?;
            // An object that escapes the matcher may be mutated despite its const binding.
            if matches!(value, Expression::ObjectExpression(_) | Expression::ArrayExpression(_))
                && !scoping.get_resolved_references(symbol).all(|reference| {
                    matches!(semantic.nodes().parent_kind(reference.node_id()), AstKind::CallExpression(call)
                        if matches!(&call.callee, Expression::Identifier(id) if id.name == "on")
                        && matches!(call.arguments.get(1), Some(Argument::Identifier(id)) if id.reference_id.get().is_some_and(|id| scoping.get_reference(id).node_id() == reference.node_id())))
                }) {
                return None;
            }
            return Some(value);
        }
    }
    None
}
fn literal(value: &Expression<'_>, semantic: &Semantic<'_>, depth: usize) -> bool {
    if depth > 32 {
        return false;
    }
    let child = |value| literal(value, semantic, depth + 1);
    match value {
        Expression::Identifier(id) => constant(id, semantic).is_some_and(child),
        Expression::StringLiteral(_) | Expression::NumericLiteral(_) | Expression::BooleanLiteral(_) | Expression::NullLiteral(_) | Expression::RegExpLiteral(_) => true,
        Expression::ArrayExpression(array) => array.elements.iter().all(|item| item.as_expression().is_some_and(child)),
        Expression::ObjectExpression(object) => object.properties.iter().all(|property| matches!(property, ObjectPropertyKind::ObjectProperty(p) if !p.computed && !p.shorthand && child(&p.value))),
        _ => false,
    }
}
