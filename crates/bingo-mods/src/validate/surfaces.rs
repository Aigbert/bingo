use super::{source::Source, *};
use bingo_sdk::render::Component;
use oxc_ast::ast::*;
use oxc_semantic::Semantic;

pub(super) fn check(call: &CallExpression<'_>, semantic: &Semantic<'_>, source: &mut Source<'_>) {
    let Some(matcher) = call.arguments.get(1).and_then(Argument::as_expression) else {
        return;
    };
    let Some(matcher) = super::hooks::resolved(matcher, semantic, 0) else {
        return;
    };
    let Expression::ObjectExpression(object) = matcher else {
        return;
    };
    let components = values(object, "component", semantic);
    let surfaces = values(object, "surface", semantic);
    for name in components {
        let Ok(component) = serde_json::from_value::<Component>(serde_json::json!(name)) else {
            continue;
        };
        for surface in &surfaces {
            if !component
                .surfaces()
                .iter()
                .any(|supported| serde_json::json!(supported) == serde_json::json!(surface))
            {
                source.finding(
                    call.span.start,
                    Rule::ComponentSurface,
                    Severity::Warning,
                    format!("{surface} never asks for {name}"),
                );
            }
        }
    }
}

fn values(object: &ObjectExpression<'_>, key: &str, semantic: &Semantic<'_>) -> Vec<String> {
    object
        .properties
        .iter()
        .find_map(|property| {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return None;
            };
            (!property.computed && property.key.static_name().as_deref() == Some(key))
                .then(|| strings(&property.value, semantic, 0))
        })
        .unwrap_or_default()
}

fn strings(value: &Expression<'_>, semantic: &Semantic<'_>, depth: usize) -> Vec<String> {
    let Some(value) = super::hooks::resolved(value, semantic, depth) else {
        return Vec::new();
    };
    match value {
        Expression::StringLiteral(value) => vec![value.value.to_string()],
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .filter_map(ArrayExpressionElement::as_expression)
            .flat_map(|value| strings(value, semantic, depth + 1))
            .collect(),
        _ => Vec::new(),
    }
}
