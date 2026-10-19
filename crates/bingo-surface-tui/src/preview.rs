//! What a permission prompt is about, drawn: a diff, a command, or a url.
//! Bounded by default so the question and its options stay on the screen;
//! ctrl+e lifts the bound.
//!
//! Diff previews use the shared `views::diff` renderer without a number gutter.

use bingo_sdk::Preview;
use ratatui::text::{Line, Span};

use crate::theme;

/// A long diff would push the options off the screen.
const DIFF_ROWS: usize = 12;
/// Heredocs and `&&` chains run long.
const COMMAND_ROWS: usize = 6;

/// The preview's rows, and how many were left out.
pub fn lines(preview: &Preview, expanded: bool) -> (Vec<Line<'static>>, usize) {
    match preview {
        Preview::Diff { unified } => bound(diff(unified), DIFF_ROWS, expanded),
        Preview::Command {
            command,
            cwd,
            description,
            ..
        } => {
            let mut rows = vec![Line::from(Span::styled(cwd.clone(), theme::dim()))];
            if let Some(description) = description {
                rows.extend(
                    description
                        .lines()
                        .map(|line| Line::from(Span::styled(line.to_string(), theme::dim()))),
                );
            }
            let metadata = rows.len();
            rows.extend(command.lines().map(command_line));
            bound(rows, COMMAND_ROWS + metadata, expanded)
        }
        Preview::Url { url } => (
            vec![Line::from(Span::styled(url.clone(), theme::link()))],
            0,
        ),
    }
}

/// The permission preview keeps the unnumbered diff frame.
pub fn diff(unified: &str) -> Vec<Line<'static>> {
    crate::views::diff::lines(unified, crate::views::diff::DiffStyle::Permission)
}

fn command_line(line: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled("$ ", theme::dim()),
        Span::styled(line.to_string(), theme::text()),
    ])
}

fn bound(rows: Vec<Line<'static>>, limit: usize, expanded: bool) -> (Vec<Line<'static>>, usize) {
    if expanded || rows.len() <= limit {
        return (rows, 0);
    }
    let hidden = rows.len() - limit;
    (rows.into_iter().take(limit).collect(), hidden)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_description_is_dim_above_the_command() {
        let (rows, hidden) = lines(
            &Preview::Command {
                command: "echo hi".into(),
                cwd: "/work".into(),
                description: Some("Print a greeting".into()),
                stakes: None,
            },
            false,
        );
        assert_eq!(hidden, 0);
        assert_eq!(
            rows[1],
            Line::from(Span::styled("Print a greeting", theme::dim()))
        );
        assert_eq!(rows[2].to_string(), "$ echo hi");
    }

    use super::*;

    fn unified(rows: usize) -> String {
        (0..rows)
            .map(|i| format!("+line {i}\n"))
            .collect::<String>()
    }

    #[test]
    fn a_long_diff_is_bounded_until_it_is_expanded() {
        let preview = Preview::Diff {
            unified: unified(20),
        };
        let (rows, hidden) = lines(&preview, false);
        assert_eq!((rows.len(), hidden), (DIFF_ROWS, 8));
        let (rows, hidden) = lines(&preview, true);
        assert_eq!((rows.len(), hidden), (20, 0));
    }

    #[test]
    fn diff_rows_take_their_colour_from_the_first_column() {
        let rows = diff("@@ -1 +1 @@\n-old\n+new\n context");
        let styles: Vec<_> = rows.iter().map(|r| r.spans[0].style).collect();
        assert_eq!(
            styles,
            vec![theme::dim(), theme::removed(), theme::added(), theme::dim()]
        );
    }

    /// One run of a row, as its text and whether it is emphasised.
    fn marked(line: &Line<'static>) -> Vec<(String, bool)> {
        line.spans
            .iter()
            .map(|span| {
                (
                    span.content.to_string(),
                    span.style
                        .add_modifier
                        .contains(ratatui::style::Modifier::BOLD),
                )
            })
            .collect()
    }

    #[test]
    fn only_the_character_run_that_changed_is_emphasised() {
        let rows = diff("@@ -1 +1 @@\n-let ready = false;\n+let ready = true;\n");
        assert_eq!(
            marked(&rows[1]),
            vec![
                ("-let ready = ".to_string(), false),
                ("fals".to_string(), true),
                ("e;".to_string(), false),
            ],
        );
        assert_eq!(
            marked(&rows[2]),
            vec![
                ("+let ready = ".to_string(), false),
                ("tru".to_string(), true),
                ("e;".to_string(), false),
            ],
        );
    }

    #[test]
    fn the_colour_still_comes_from_the_column_and_not_from_the_words() {
        let rows = diff("-old line\n+new line\n");
        assert!(
            rows[0]
                .spans
                .iter()
                .all(|s| s.style.fg == theme::removed().fg)
        );
        assert!(
            rows[1]
                .spans
                .iter()
                .all(|s| s.style.fg == theme::added().fg)
        );
    }

    #[test]
    fn a_row_nothing_replaced_is_left_plain() {
        let rows = diff("@@ -1,2 +1,1 @@\n-first\n-second\n+first\n");
        assert_eq!(rows.len(), 4);
        assert_eq!(
            marked(&rows[2]),
            vec![("-second".to_string(), false)],
            "the second removal was replaced by nothing"
        );
        assert!(
            marked(&rows[1]).iter().all(|(_, bold)| !*bold),
            "and the row that survived unchanged has nothing to point at"
        );
    }

    #[test]
    fn a_file_header_is_never_read_as_a_removal() {
        let rows = diff("--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1 +1 @@\n-a\n+b\n");
        assert_eq!(rows[0].spans[0].style, theme::dim());
        assert_eq!(rows[1].spans[0].style, theme::dim());
        assert_eq!(rows[3].spans[0].style, theme::removed());
    }

    #[test]
    fn a_command_preview_shows_its_directory_and_the_line() {
        let (rows, _) = lines(
            &Preview::Command {
                command: "ls -la".into(),
                description: None,
                stakes: None,
                cwd: "/tmp".into(),
            },
            false,
        );
        assert_eq!(
            rows.iter().map(|r| r.to_string()).collect::<Vec<_>>(),
            vec!["/tmp", "$ ls -la"]
        );
    }
}
