use super::{source::Source, *};
use oxc_ast::{AstKind, ast::*};
use oxc_semantic::Semantic;

pub(super) fn check(semantic: &Semantic<'_>, source: &mut Source<'_>) {
    for node in semantic.nodes().iter() {
        let AstKind::JSXOpeningElement(element) = node.kind() else {
            continue;
        };
        let name = match &element.name {
            JSXElementName::Identifier(name) => name.name.as_str(),
            JSXElementName::IdentifierReference(name) => name.name.as_str(),
            _ => continue,
        };
        let Some(table) = bingo_sdk::view::table::ELEMENTS
            .iter()
            .find(|table| table.kind == name)
        else {
            continue;
        };
        for property in &element.attributes {
            let JSXAttributeItem::Attribute(property) = property else {
                continue;
            };
            let JSXAttributeName::Identifier(name) = &property.name else {
                continue;
            };
            if !table.props.contains(&name.name.as_str()) {
                source.finding(
                    property.span.start,
                    Rule::PropSubset,
                    Severity::Warning,
                    format!("terminal ignores {}.{}", table.kind, name.name),
                );
            }
        }
    }
}
