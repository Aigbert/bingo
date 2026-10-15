//! Text shared by schedule listings and confirmation views.
use std::borrow::Cow;

/// Reveal characters a terminal drops or uses to reorder visible text.
/// A borrowed result means the text can be shown without escapes.
pub fn visible(text: &str) -> Cow<'_, str> {
    if !text.chars().any(hidden) {
        return Cow::Borrowed(text);
    }
    let mut shown = String::new();
    for character in text.chars() {
        if hidden(character) {
            shown.extend(character.escape_unicode());
        } else {
            shown.push(character);
        }
    }
    Cow::Owned(shown)
}

fn hidden(character: char) -> bool {
    (character.is_control() && !matches!(character, '\n' | '\t'))
        || matches!(character, '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

/// Bind the exact text and optional mode, with length-delimited fields.
pub fn confirmation_digest(text: &str, mode: Option<&str>) -> String {
    let mut bytes = b"bingo.schedule.confirm.v1\0".to_vec();
    bytes.extend_from_slice(&(text.len() as u64).to_be_bytes());
    bytes.extend_from_slice(text.as_bytes());
    bytes.push(u8::from(mode.is_some()));
    if let Some(mode) = mode {
        bytes.extend_from_slice(mode.as_bytes());
    }
    bingo_digest::hex(&bingo_digest::digest(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_digest_pins_exact_text_and_optional_mode() {
        assert_eq!(
            confirmation_digest("hello", None),
            "c05e7dfa34c3ba8e3a9c414b933f4e4e55919a709ea8c2eb94ac5041dd55f8bc"
        );
        let cases = [
            ("hello", None),
            ("hello", Some("")),
            ("hello", Some("plan")),
            ("hello\n", None),
            ("hell", Some("o")),
        ];
        for (index, (text, mode)) in cases.iter().enumerate() {
            for (other_text, other_mode) in &cases[index + 1..] {
                assert_ne!(
                    confirmation_digest(text, *mode),
                    confirmation_digest(other_text, *other_mode)
                );
            }
        }
    }

    #[test]
    fn controls_and_direction_marks_are_visible_but_lines_and_tabs_stay() {
        assert!(matches!(visible("one\ntwo\t日本語"), Cow::Borrowed(_)));
        for character in (0..=0x9f)
            .filter_map(char::from_u32)
            .filter(|c| c.is_control())
            .chain(['\u{200e}', '\u{200f}'])
            .chain(('\u{202a}'..='\u{202e}').chain('\u{2066}'..='\u{2069}'))
        {
            if matches!(character, '\n' | '\t') {
                continue;
            }
            let raw = format!("a{character}b");
            assert_eq!(
                visible(&raw),
                format!("a\\u{{{:x}}}b", u32::from(character))
            );
        }
    }
}
