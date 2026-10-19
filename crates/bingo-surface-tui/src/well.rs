//! The inset frame and cut around an owner's output.
use crate::{clock::Now, layout::grid, theme, views};
use bingo_sdk::{ContentPart, ToolOutput, View, component::Tier};
use ratatui::text::{Line, Span};

pub const KEEP: usize = 12;

pub fn content(
    output: Option<&ToolOutput>,
    tier: Tier,
    skipped: bool,
    width: usize,
    now: Now,
) -> views::Drawing {
    let display = output.and_then(|output| output.display.as_ref());
    let (drawing, cut) = match display {
        Some(view) => (
            views::marked(
                view,
                width,
                &views::Marks {
                    diff: views::diff::DiffStyle::Well,
                    ..views::Marks::at(now)
                },
            ),
            Cut::of(view),
        ),
        None => (fallback(output, skipped), Cut::Tier),
    };
    drawing.map_lines(|lines| cut.apply(lines, tier))
}

enum Cut {
    Whole,
    Code(Option<u32>, Option<u32>),
    Tier,
}
impl Cut {
    fn of(view: &View) -> Self {
        match view {
            View::Diff { .. } => Self::Whole,
            View::Code { start, of, .. } => Self::Code(*start, *of),
            _ => Self::Tier,
        }
    }
    fn apply(self, lines: Vec<Line<'static>>, tier: Tier) -> Vec<Line<'static>> {
        match self {
            Self::Whole => lines,
            Self::Code(start, of) => code_cut(lines, start, of),
            Self::Tier => cut(lines, tier == Tier::Lookup, KEEP),
        }
    }
}

fn fallback(output: Option<&ToolOutput>, skipped: bool) -> views::Drawing {
    let text = output
        .map(|output| {
            output
                .parts
                .iter()
                .filter_map(ContentPart::as_text)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    if text.is_empty() {
        vec![Line::from(Span::styled(
            if skipped { "did not run" } else { "no output" },
            theme::dim(),
        ))]
        .into()
    } else {
        text_lines(&text).into()
    }
}

pub fn inner(width: usize) -> usize {
    width.saturating_sub(grid::VX + grid::RM + 4)
}

pub fn text_lines(text: &str) -> Vec<Line<'static>> {
    text.lines()
        .map(|text| {
            let lower = text.to_lowercase();
            let bad = ["error", "failed", "panicked"]
                .iter()
                .any(|word| lower.contains(word));
            Line::from(Span::styled(
                text.to_owned(),
                if bad { theme::bad() } else { theme::ash() },
            ))
        })
        .collect()
}

pub fn cut(mut lines: Vec<Line<'static>>, head: bool, keep: usize) -> Vec<Line<'static>> {
    let more = lines.len().saturating_sub(keep);
    if more == 0 {
        return lines;
    }
    let note = Line::from(Span::styled(
        format!(
            "{} {more} {} lines",
            theme::glyphs::get("ell"),
            if head { "more" } else { "earlier" }
        ),
        theme::dim(),
    ));
    if head {
        lines.truncate(keep);
        lines.push(note);
    } else {
        lines.drain(..more);
        lines.insert(0, note);
    }
    lines
}

fn code_cut(
    mut lines: Vec<Line<'static>>,
    start: Option<u32>,
    of: Option<u32>,
) -> Vec<Line<'static>> {
    let shown = lines.len().min(KEEP);
    let more = of
        .map(|of| of.saturating_sub(start.unwrap_or(1).saturating_sub(1)) as usize)
        .unwrap_or(lines.len())
        .saturating_sub(shown);
    lines.truncate(shown);
    if more > 0 {
        lines.push(Line::from(Span::styled(
            format!("{} {more} more lines", theme::glyphs::get("ell")),
            theme::dim(),
        )));
    }
    lines
}

pub fn frame(lines: Vec<Line<'static>>, width: usize) -> Vec<Line<'static>> {
    let width = width.saturating_sub(grid::VX + grid::RM);
    let blank = Line::default();
    std::iter::once(blank.clone())
        .chain(lines)
        .chain(std::iter::once(blank))
        .map(|line| framed(line, width))
        .collect()
}

fn framed(line: Line<'static>, width: usize) -> Line<'static> {
    let ground = theme::ground("well").unwrap_or_default().patch(line.style);
    let line = grid::fit(line, width.saturating_sub(4));
    let mut inside = Line::from("  ");
    inside.spans.extend(line.spans);
    let mut inside = grid::pad_to(inside, width);
    for span in &mut inside.spans {
        span.style = ground.patch(span.style);
    }
    inside.spans.insert(0, Span::raw(" ".repeat(grid::VX)));
    inside
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn head_and_tail_keep_twelve_lines_and_name_the_cut() {
        let lines: Vec<_> = (1..=15).map(|n| Line::from(n.to_string())).collect();
        let head = cut(lines.clone(), true, KEEP);
        assert_eq!(head.len(), 13);
        assert_eq!(head[0].to_string(), "1");
        assert_eq!(head[11].to_string(), "12");
        assert!(head[12].to_string().ends_with("3 more lines"));
        let tail = cut(lines, false, KEEP);
        assert_eq!(tail.len(), 13);
        assert!(tail[0].to_string().ends_with("3 earlier lines"));
        assert_eq!(tail[1].to_string(), "4");
        assert_eq!(tail[12].to_string(), "15");
    }
}

#[cfg(test)]
mod owner_tests {
    use super::*;
    use crate::test_support::{later, scene, still};
    use serde_json::json;

    #[test]
    fn a_declared_view_keeps_nested_style_ground_and_motion() {
        theme::with(crate::painted::truecolor(), || {
            let view: View = serde_json::from_value(json!({
                "kind":"Box", "children":[{"kind":"Text", "italic":true,
                "color":"good", "ground":"you", "children":["owner"],
                "motion":{"kind":"breathe", "with":"coal", "periodMs":1800,"depth":1.0}}]
            }))
            .unwrap();
            let output = ToolOutput {
                display: Some(view),
                ..ToolOutput::text("model only")
            };
            let now = scene().1;
            let first = content(Some(&output), Tier::Lookup, false, 40, now);
            assert!(first.next.is_some());
            let framed = first.map_lines(|lines| frame(lines, 51));
            assert!(framed.next.is_some());
            let span = framed.lines[1]
                .spans
                .iter()
                .find(|s| s.content == "owner")
                .unwrap();
            assert_eq!(span.style.bg, theme::ground("you").unwrap().bg);
            assert!(
                span.style
                    .add_modifier
                    .contains(theme::italic().add_modifier)
            );
            assert!(
                !framed
                    .lines
                    .iter()
                    .any(|line| line.to_string().contains("model only"))
            );
            let moving = content(Some(&output), Tier::Lookup, false, 40, later(now, 900));
            assert_ne!(frame(moving.lines, 51), framed.lines);
            assert!(
                content(Some(&output), Tier::Lookup, false, 0, still(now))
                    .next
                    .is_none()
            );
            assert!(
                content(Some(&output), Tier::Lookup, false, 0, now)
                    .next
                    .is_some()
            );
        });
    }

    #[test]
    fn code_keeps_its_source_start_through_the_lookup_cut() {
        let output = ToolOutput {
            display: Some(View::Code {
                language: None,
                start: Some(42),
                of: None,
                source: (0..15).map(|n| format!("row {n}\n")).collect(),
            }),
            ..ToolOutput::text("model only")
        };
        let drawing = content(Some(&output), Tier::Lookup, false, 40, scene().1);
        assert_eq!(drawing.lines[0].to_string(), "  42   row 0");
        assert_eq!(drawing.lines[11].to_string(), "  53   row 11");
        assert!(drawing.lines[12].to_string().ends_with("3 more lines"));
    }

    #[test]
    fn empty_and_skipped_outputs_name_what_happened() {
        for (skipped, expected) in [(false, "no output"), (true, "did not run")] {
            let drawing = content(None, Tier::Lookup, skipped, 40, scene().1);
            assert_eq!(drawing.lines[0].to_string(), expected);
        }
    }
}

#[cfg(test)]
mod cut_tests {
    use super::*;
    use crate::test_support::scene;

    #[test]
    fn a_code_well_keeps_its_head_and_counts_the_rest_of_the_file() {
        let output = ToolOutput {
            display: Some(View::Code {
                language: None,
                source: (0..15).map(|n| format!("row {n}\n")).collect(),
                start: Some(42),
                of: Some(80),
            }),
            ..ToolOutput::text("")
        };
        let drawing = content(Some(&output), Tier::Change, false, 40, scene().1);
        assert_eq!(drawing.lines[0].to_string(), "  42   row 0");
        assert_eq!(drawing.lines[11].to_string(), "  53   row 11");
        assert!(drawing.lines[12].to_string().ends_with("27 more lines"));
    }

    #[test]
    fn a_box_obeys_the_tier_cut() {
        let output = ToolOutput {
            display: Some(View::column(
                (1..=15).map(|n| View::text(n.to_string())).collect(),
            )),
            ..ToolOutput::text("")
        };
        let drawing = content(Some(&output), Tier::Lookup, false, 40, scene().1);
        assert_eq!(drawing.lines.len(), 13);
        assert_eq!(drawing.lines[0].to_string().trim_end(), "1");
        assert!(drawing.lines[12].to_string().ends_with("3 more lines"));
    }
}
