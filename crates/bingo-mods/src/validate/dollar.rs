use super::{source::Source, *};
use oxc_ast::{AstKind, ast::*};
use oxc_semantic::Semantic;
use oxc_span::GetSpan;

pub(super) fn chain(expression: &Expression<'_>, semantic: &Semantic<'_>) -> Option<String> {
    match expression {
        Expression::Identifier(id) if is_dollar(id, semantic) => Some(String::new()),
        Expression::StaticMemberExpression(member) => {
            chain(&member.object, semantic).map(|prefix| {
                if prefix.is_empty() {
                    member.property.name.to_string()
                } else {
                    format!("{prefix}.{}", member.property.name)
                }
            })
        }
        _ => None,
    }
}
pub(super) fn check(semantic: &Semantic<'_>, source: &mut Source<'_>) {
    for node in semantic.nodes().iter() {
        match node.kind() {
            AstKind::ComputedMemberExpression(member)
                if chain(&member.object, semantic)
                    .is_some_and(|name| !crate::dollar::table::data_member(&name)) =>
            {
                source.finding(
                    member.span.start,
                    Rule::DollarStatic,
                    Severity::Error,
                    "$ members must be static",
                )
            }
            AstKind::CallExpression(call) => {
                if let Some(name) = chain(&call.callee, semantic) {
                    if !crate::dollar::table::data_member(&name) {
                        source.output.calls.insert(name.clone());
                        super::uses::call(&name, call, source);
                    }
                } else {
                    super::helpers::calls(call, semantic, source);
                }
            }
            AstKind::IdentifierReference(id) if is_dollar(id, semantic) => {
                let parent = semantic.nodes().parent_kind(node.id());
                if !matches!(
                    parent,
                    AstKind::StaticMemberExpression(_)
                        | AstKind::ComputedMemberExpression(_)
                        | AstKind::CallExpression(_)
                ) {
                    source.finding(
                        id.span.start,
                        Rule::DollarStatic,
                        Severity::Error,
                        "$ cannot be aliased or stored",
                    );
                }
            }
            AstKind::StaticMemberExpression(member)
                if chain(&member.object, semantic).is_some() =>
            {
                let parent = semantic.nodes().parent_kind(node.id());
                let is_chain = matches!(
                    parent,
                    AstKind::StaticMemberExpression(_) | AstKind::ComputedMemberExpression(_)
                );
                let is_called = matches!(parent, AstKind::CallExpression(call) if call.callee.span() == member.span());
                let data = chain(&member.object, semantic).is_some_and(|prefix| {
                    crate::dollar::table::data_member(&format!("{prefix}.{}", member.property.name))
                });
                if !is_chain && !is_called && !data {
                    source.finding(
                        member.span().start,
                        Rule::DollarStatic,
                        Severity::Error,
                        "$ members cannot be aliased or stored",
                    );
                }
            }
            _ => (),
        }
    }
}

fn is_dollar(id: &IdentifierReference<'_>, semantic: &Semantic<'_>) -> bool {
    if id.name == "$" {
        return true;
    }
    let Some(reference) = id.reference_id.get() else {
        return false;
    };
    let Some(symbol) = semantic.scoping().get_reference(reference).symbol_id() else {
        return false;
    };
    let declaration = semantic.scoping().symbol_declaration(symbol);
    let Some(call) = semantic
        .nodes()
        .ancestor_kinds(declaration)
        .find_map(|kind| {
            if let AstKind::CallExpression(call) = kind {
                Some(call)
            } else {
                None
            }
        })
    else {
        return false;
    };
    let parameter = call.arguments.iter().any(|argument| {
        let params = match argument { Argument::ArrowFunctionExpression(function) => &function.params, Argument::FunctionExpression(function) => &function.params, _ => return false };
        params.items.first().is_some_and(|parameter| matches!(&parameter.pattern, BindingPattern::BindingIdentifier(binding) if binding.symbol_id.get() == Some(symbol)))
    });
    parameter
        && (matches!(&call.callee, Expression::Identifier(id) if id.name == "on")
            || chain(&call.callee, semantic).is_some_and(|name| {
                matches!(
                    name.as_str(),
                    "command.register" | "tool.register" | "clock.after" | "clock.every"
                )
            }))
}
