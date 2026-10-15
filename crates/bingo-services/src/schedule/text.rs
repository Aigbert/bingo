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

#[cfg(test)]
mod tests {
    use super::*;

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
