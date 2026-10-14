use super::{source::Source, *};
use oxc_ast::{AstKind, ast::*};
use oxc_semantic::Semantic;
use oxc_span::GetSpan;

pub(super) fn chain(expression: &Expression<'_>) -> Option<String> {
    match expression {
        Expression::Identifier(id) if id.name == "$" => Some(String::new()),
        Expression::StaticMemberExpression(member) => chain(&member.object).map(|prefix| {
            if prefix.is_empty() {
                member.property.name.to_string()
            } else {
                format!("{prefix}.{}", member.property.name)
            }
        }),
        _ => None,
    }
}
pub(super) fn check(semantic: &Semantic<'_>, source: &mut Source<'_>) {
    for node in semantic.nodes().iter() {
        match node.kind() {
            AstKind::ComputedMemberExpression(member) if chain(&member.object).is_some() => source
                .finding(
                    member.span.start,
                    Rule::DollarStatic,
                    Severity::Error,
                    "$ members must be static",
                ),
            AstKind::CallExpression(call) => {
                if let Some(name) = chain(&call.callee) {
                    source.output.calls.insert(name.clone());
                    super::uses::call(&name, call, source);
                }
            }
            AstKind::IdentifierReference(id) if id.name == "$" => {
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
            AstKind::StaticMemberExpression(member) if chain(&member.object).is_some() => {
                let parent = semantic.nodes().parent_kind(node.id());
                if !matches!(
                    parent,
                    AstKind::StaticMemberExpression(_)
                        | AstKind::ComputedMemberExpression(_)
                        | AstKind::CallExpression(_)
                ) {
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
