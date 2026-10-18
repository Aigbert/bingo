//! Resolve the row declared by a tool, for every surface.
use crate::{ExternalCall, Reads, ToolCall, ToolLook};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use unicode_width::UnicodeWidthChar;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RowLook {
    pub mark: char,
    pub verb: String,
    pub target: Option<(String, Reads)>,
}

/// Resolve a declaration, or an ACP call's kind and title, without naming tools.
/// Fixture cases: declared, ascii_twin, ascii_mark, wide_mark, zero_width,
/// control, non_ascii, invalid_twin, unknown, unknown_ascii, missing_target,
/// non_string_target, short_string, no_string, command, text, acp, acp_ascii,
/// colon_name. A fallback string is nonempty, single-line and at most 120 chars.
pub fn row(name: &str, input: &Value, declared: Option<&ToolLook>, ascii: bool) -> RowLook {
    let external = declared
        .is_none()
        .then(|| external_look(name, input))
        .flatten();
    let declared = declared.or(external.as_ref());
    RowLook {
        mark: mark(
            declared.map(|look| {
                if ascii {
                    look.ascii.unwrap_or(look.mark)
                } else {
                    look.mark
                }
            }),
            ascii,
        ),
        verb: declared
            .map(|look| look.verb.clone())
            .unwrap_or_else(|| name.to_lowercase()),
        target: target(input, declared),
    }
}

fn external_look(name: &str, input: &Value) -> Option<ToolLook> {
    if !name.contains(':') {
        return None;
    }
    ExternalCall::from_input(&ToolCall {
        call_id: String::new(),
        name: name.into(),
        input: input.clone(),
    })
    .map(|call| call.kind.look())
}

/// Resolve a one-cell mark for the surface's character set.
/// Pass the declared ASCII twin when one is available under ASCII.
pub fn mark(declared: Option<char>, ascii: bool) -> char {
    declared
        .filter(|mark| mark.width() == Some(1) && (!ascii || mark.is_ascii()))
        .unwrap_or(if ascii { '-' } else { '•' })
}

fn target(input: &Value, declared: Option<&ToolLook>) -> Option<(String, Reads)> {
    if let Some(target) = declared.and_then(|look| look.target.as_ref())
        && let Some(value) = input.get(&target.field).and_then(Value::as_str)
    {
        return Some((value.into(), target.reads));
    }
    input
        .as_object()?
        .values()
        .filter_map(Value::as_str)
        .find(|s| !s.is_empty() && !s.contains(['\n', '\r']) && s.chars().count() <= 120)
        .map(|s| (s.into(), Reads::Text))
}
