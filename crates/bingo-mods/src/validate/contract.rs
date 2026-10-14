use super::*;
use oxc_allocator::Allocator;
use oxc_ast::{AstKind, ast::*};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

pub(super) fn read(
    root: &Path,
    name: &str,
    output: &mut Validation,
) -> Result<Option<BTreeSet<String>>, String> {
    let path = root.join("types/index.d.ts");
    if !path.exists() {
        output.findings.push(Finding {
            file: "types/index.d.ts".into(),
            line: 1,
            column: 1,
            rule: Rule::StateContract,
            severity: Severity::Warning,
            text: "no PluginState contract".into(),
        });
        return Ok(None);
    }
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(Some(declared(&text, name)))
}
fn declared(text: &str, name: &str) -> BTreeSet<String> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, text, SourceType::d_ts()).parse();
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    let mut keys = BTreeSet::new();
    for node in built.semantic.nodes().iter() {
        let AstKind::TSInterfaceDeclaration(interface) = node.kind() else {
            continue;
        };
        if interface.id.name != "PluginState" {
            continue;
        }
        for property in &interface.body.body {
            let TSSignature::TSPropertySignature(property) = property else {
                continue;
            };
            if property.key.static_name().as_deref() != Some(name) {
                continue;
            }
            let Some(annotation) = &property.type_annotation else {
                continue;
            };
            let TSType::TSTypeLiteral(literal) = &annotation.type_annotation else {
                continue;
            };
            keys.extend(literal.members.iter().filter_map(|member| {
                if let TSSignature::TSPropertySignature(property) = member {
                    property.key.static_name().map(|key| key.into_owned())
                } else {
                    None
                }
            }));
        }
    }
    keys
}
