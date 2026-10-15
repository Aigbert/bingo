//! A text shown whole before a person confirms it: every line that will
//! fire, never its first line alone (ADR-0094 §1). A text the pane cannot
//! hold is not confirmed from the pane.
use crate::draw::line;
use bingo_sdk::View;
use bingo_services::schedule::Row;

/// Where the text sits under the line that introduces it.
const INDENT: &str = "   ";
/// The rows the pane spends on anything but the text: heading, keys, the
/// line naming the entry, a trouble badge, the rule and the footer.
const AROUND: u16 = 6;

/// The pane's body while `row` waits for its confirmation, and whether it
/// shows the whole text, which is what arms ⏎.
pub(crate) fn body(row: &Row, width: u16, height: u16) -> (Vec<View>, bool) {
    let visible = bingo_services::schedule::visible(&row.text);
    let safe = matches!(visible, std::borrow::Cow::Borrowed(_));
    let rows = chunks(&visible, usize::from(width).saturating_sub(INDENT.len()));
    if rows.len() > usize::from(height.saturating_sub(AROUND)) {
        let said = format!(
            " {}'s text is {} rows here, more than this pane holds",
            row.id,
            rows.len()
        );
        return (vec![line(said, true)], false);
    }
    let named = format!(" {} · {} · when it fires it says:", row.id, row.spec);
    let text = rows
        .into_iter()
        .map(|row| line(format!("{INDENT}{row}"), false));
    (
        std::iter::once(line(named, false)).chain(text).collect(),
        safe,
    )
}

/// `text` cut into rows no wider than `width` cells. A cell count is
/// estimated high — every non-ASCII character as two, a tab as eight — so a
/// row is never clipped and the count is never short.
pub(crate) fn chunks(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    text.split('\n')
        .flat_map(|line| split(line, width))
        .collect()
}

fn split(line: &str, width: usize) -> Vec<String> {
    let mut rows = vec![String::new()];
    let mut used = 0;
    for character in line.chars() {
        let cells = cells(character);
        if used > 0 && used + cells > width {
            rows.push(String::new());
            used = 0;
        }
        if let Some(row) = rows.last_mut() {
            row.push(character);
        }
        used += cells;
    }
    rows
}

fn cells(character: char) -> usize {
    match character {
        '\t' => 8,
        character if character.is_ascii() => 1,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(text: &str) -> Row {
        Row {
            id: "abcd".into(),
            spec: "every 2h".into(),
            next_fire: None,
            enabled: true,
            by: "unconfirmed".into(),
            mode: None,
            text: text.into(),
            session: None,
        }
    }

    #[test]
    fn every_line_is_a_row_and_a_long_one_is_cut_not_lost() {
        assert_eq!(
            chunks("/guard\nrm -rf target", 40),
            ["/guard", "rm -rf target"]
        );
        assert_eq!(chunks("abcdefg", 3), ["abc", "def", "g"]);
        assert_eq!(
            chunks("日本語", 4),
            ["日本", "語"],
            "wide characters count twice"
        );
        assert_eq!(chunks("a\r\n\nb", 9), ["a\r", "", "b"], "blank lines stay");
    }

    #[test]
    fn a_text_the_pane_holds_is_shown_whole_and_arms_the_confirmation() {
        let (body, armed) = body(&row("/guard\nrm -rf target"), 80, 24);
        assert!(armed);
        let shown: Vec<String> = body.iter().map(View::fold).collect();
        assert_eq!(
            shown,
            [
                " abcd · every 2h · when it fires it says:",
                "   /guard",
                "   rm -rf target"
            ]
        );
    }

    #[test]
    fn a_text_the_pane_cannot_hold_does_not_arm_it() {
        let long = vec!["line"; 30].join("\n");
        let (body, armed) = body(&row(&long), 80, 24);
        assert!(!armed);
        assert_eq!(body.len(), 1);
        assert!(body[0].fold().contains("30 rows"), "{}", body[0].fold());
    }

    /// ⏎ is offered a confirmation only when the pane showed all of it.
    fn offered(text: &str) -> serde_json::Value {
        let row = row(text);
        let listing = bingo_services::schedule::Listing {
            entries: vec![row.clone()],
            ..Default::default()
        };
        let confirming = crate::Confirmation::Confirm(row);
        let tree = crate::draw::tree(
            &listing,
            0,
            Some(&confirming),
            80,
            24,
            jiff::Timestamp::UNIX_EPOCH,
        );
        let View::Box { layout, .. } = tree else {
            panic!("the pane is a box");
        };
        layout.on_key.expect("keys").args["confirming"].clone()
    }

    #[test]
    fn invisible_text_is_escaped_and_never_arms_enter() {
        for text in ["a\rb", "a\u{1b}b", "a\u{202e}b", "a\u{2069}b"] {
            let (views, armed) = body(&row(text), 80, 24);
            assert!(!armed);
            assert!(
                views
                    .iter()
                    .map(View::fold)
                    .collect::<String>()
                    .contains("\\u{")
            );
            assert!(offered(text).is_null());
        }
    }

    #[test]
    fn enter_is_armed_only_for_a_text_shown_whole() {
        assert!(!offered("/guard\nrm -rf target").is_null());
        assert!(offered(&vec!["line"; 30].join("\n")).is_null());
    }
}
