use super::{source::Source, *};
use oxc_ast::ast::CallExpression;

pub(super) fn call(name: &str, call: &CallExpression<'_>, source: &mut Source<'_>) {
    let Some((noun, method)) = name.split_once('.') else {
        return;
    };
    let unsafe_call = crate::dollar::table::ROWS
        .iter()
        .any(|row| row.noun == noun && row.method == method && row.unsafe_call);
    if unsafe_call && !source.output.uses.contains(noun) {
        source.finding(
            call.span.start,
            Rule::Uses,
            Severity::Error,
            format!("unsafe call {name} requires uses: {noun}"),
        );
    }
}
pub(super) fn unused(output: &mut Validation) {
    let unused: Vec<_> = output
        .uses
        .iter()
        .filter(|noun| {
            !output
                .calls
                .iter()
                .any(|call| call.starts_with(&format!("{noun}.")))
        })
        .cloned()
        .collect();
    for noun in unused {
        output.findings.push(Finding {
            file: "plugin.json".into(),
            line: 1,
            column: 1,
            rule: Rule::Uses,
            severity: Severity::Warning,
            text: format!("uses {noun} is never called"),
        });
    }
}
