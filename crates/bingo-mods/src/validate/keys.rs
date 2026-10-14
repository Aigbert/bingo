use super::{source::Source, *};
use oxc_ast::{AstKind, ast::*};
use oxc_semantic::Semantic;

pub(super) fn check(semantic: &Semantic<'_>, source: &mut Source<'_>) {
    for node in semantic.nodes().iter() {
        let AstKind::CallExpression(call) = node.kind() else {
            continue;
        };
        let name = super::dollar::chain(&call.callee).or_else(|| {
            matches!(&call.callee, Expression::Identifier(id) if id.name == "atom")
                .then(|| "state.atom".into())
        });
        let Some(name) = name else {
            continue;
        };
        let Some((noun, method)) = name.split_once('.') else {
            continue;
        };
        if !matches!(noun, "state" | "store" | "env") || method == "keys" {
            continue;
        }
        let key = call
            .arguments
            .first()
            .and_then(|argument| key(argument, noun));
        if let Some(value) = key {
            if noun == "state" && source.declared.is_some_and(|keys| !keys.contains(value)) {
                source.finding(
                    call.span.start,
                    Rule::StateContract,
                    Severity::Error,
                    format!(
                        "state key {} is absent from PluginState['{}']",
                        value, source.plugin
                    ),
                );
            }
            if noun != "env" {
                source.output.keys.insert(format!("{noun}:{value}"));
            }
        } else {
            let rule = if noun == "env" {
                Rule::EnvName
            } else {
                Rule::StateKey
            };
            let text = if noun == "env" {
                "env name must be a string literal"
            } else {
                "state key must be a string literal"
            };
            source.finding(call.span.start, rule, Severity::Error, text);
        }
    }
}

fn key<'a>(argument: &'a Argument<'a>, noun: &str) -> Option<&'a str> {
    if let Argument::StringLiteral(value) = argument {
        return Some(value.value.as_str());
    }
    if noun == "env"
        && let Argument::ObjectExpression(object) = argument
    {
        return object.properties.iter().find_map(|property| {
            if let ObjectPropertyKind::ObjectProperty(property) = property
                && !property.computed
                && property.key.static_name().as_deref() == Some("name")
                && let Expression::StringLiteral(value) = &property.value
            {
                Some(value.value.as_str())
            } else {
                None
            }
        });
    }
    None
}
