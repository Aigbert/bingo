//! Find owns only its query, current match and where the stream was parked when it opened.
use crate::{
    clock::Now,
    focus::Focus,
    keys,
    layout::{Canvas, Layout},
    theme,
    ui::Ui,
};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
};
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hit {
    pub line: usize,
    pub column: usize,
    pub width: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Search {
    pub query: String,
    pub current: Option<usize>,
    /// Where the stream was parked when find opened, which esc gives back.
    pub from: crate::scroll::Scroll,
}

impl Search {
    pub fn open() -> Self {
        Self::default()
    }
    pub fn type_char(&mut self, c: char) {
        self.query.push(c);
        self.current = None;
    }
    pub fn delete(&mut self) {
        self.query.pop();
        self.current = None;
    }
    pub fn step(&mut self, hits: &[Hit], by: isize) {
        self.current = self
            .at(hits)
            .map(|at| at.saturating_add_signed(by).min(hits.len() - 1));
    }
    pub fn at(&self, hits: &[Hit]) -> Option<usize> {
        (!hits.is_empty()).then(|| self.current.unwrap_or(hits.len() - 1).min(hits.len() - 1))
    }
}

pub fn open(ui: &mut Ui, now: Now) {
    if ui.shown.blocks.height() == 0 {
        ui.notify(bingo_sdk::Level::Info, "nothing to find yet", now.instant);
        return;
    }
    ui.focus = Focus::Search(Search {
        from: ui.scroll,
        ..Search::open()
    });
}

pub fn key(ui: &mut Ui, key: crossterm::event::KeyEvent, now: Now) -> Vec<crate::effect::Effect> {
    let lines = drawn(&ui.shown);
    let Some(search) = ui.focus.search_mut() else {
        return Vec::new();
    };
    match keys::find(key) {
        Some(keys::Find::Back) => back(ui),
        Some(keys::Find::Keep) => {
            ui.focus = Focus::Composer;
            ui.scroll = crate::scroll::Scroll::park(ui.shown.top, now.instant);
        }
        Some(keys::Find::Move(by)) => {
            let hits = hits(&lines, &search.query);
            search.step(&hits, by);
        }
        Some(keys::Find::Type(c)) => search.type_char(c),
        Some(keys::Find::Delete) => search.delete(),
        None => {}
    }
    Vec::new()
}

/// Closes find and gives back where the stream was when it opened.
pub fn back(ui: &mut Ui) {
    if let Some(search) = ui.focus.search() {
        ui.scroll = search.from;
    }
    ui.focus = Focus::Composer;
}

pub fn drawn(frame: &Layout) -> Vec<String> {
    frame
        .blocks
        .window(0, frame.blocks.height())
        .into_iter()
        .map(|line| {
            crate::layout::grid::fit(line, frame.regions.transcript.width as usize).to_string()
        })
        .collect()
}

pub fn top(search: &Search, frame: &Layout, height: usize) -> Option<usize> {
    let hits = hits(&drawn(frame), &search.query);
    let hit = hits.get(search.at(&hits)?)?;
    Some(
        hit.line
            .saturating_sub(height / 2)
            .min(frame.height.saturating_sub(height)),
    )
}

pub fn demand(ui: &Ui) -> Option<u16> {
    ui.focus.search().map(|_| 1)
}

pub fn draw(search: &Search, ui: &Ui, frame: &mut Layout, area: Rect, now: Now) {
    let hits = hits(&drawn(frame), &search.query);
    let tally = if search.query.is_empty() {
        String::new()
    } else {
        search
            .at(&hits)
            .map(|at| format!("{} of {}", at + 1, hits.len()))
            .unwrap_or_else(|| "no match".into())
    };
    let mut line = Line::from(vec![
        Span::raw(" ".repeat(crate::layout::grid::TX)),
        Span::styled("find  ", theme::dim()),
        Span::styled(search.query.clone(), theme::text()),
    ]);
    let rest = ui.keyed_at + std::time::Duration::from_secs(10);
    let now = Now {
        motion: now.motion && now.instant < rest,
        ..now
    };
    let caret = crate::views::Drawing {
        lines: vec![Line::from(Span::styled(
            theme::glyphs::get("caret"),
            theme::caret_ink(now, "glow", "you").unwrap_or_default(),
        ))],
        next: now
            .motion
            .then_some((now.instant + crate::clock::FRAME).min(rest)),
    };
    let caret_width = line.width();
    line.spans
        .extend(caret.lines.into_iter().flat_map(|line| line.spans));
    if search.query.is_empty() {
        line.spans
            .push(Span::styled("in this conversation", theme::dim()));
    }
    let room = (area.width as usize).saturating_sub(2 + tally.width());
    line = crate::layout::grid::pad_to(line, room);
    line.spans.push(Span::styled(tally, theme::ash()));
    if area.height > 0 && caret_width < area.width as usize {
        frame.wake(caret.next);
    }
    frame.render_widget(Paragraph::new(line), area);
}

/// Every occurrence of `query` in these lines, in reading order.
pub fn hits(lines: &[String], query: &str) -> Vec<Hit> {
    if query.is_empty() {
        return Vec::new();
    }
    let needle = query.to_lowercase();
    lines
        .iter()
        .enumerate()
        .flat_map(|(line, text)| in_line(line, text, &needle))
        .collect()
}

fn in_line(line: usize, text: &str, needle: &str) -> Vec<Hit> {
    let haystack = text.to_lowercase();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(found) = haystack[from..].find(needle) {
        let start = from + found;
        out.push(Hit {
            line,
            // Cells, not bytes: a highlight is drawn over columns.
            column: haystack[..start].width(),
            width: needle.width(),
        });
        from = start + needle.len().max(1);
    }
    out
}

pub fn mark(frame: &mut Canvas, area: Rect, top: usize, search: &Search, lines: &[String]) {
    let hits = hits(lines, &search.query);
    let current = search.at(&hits);
    for (i, hit) in hits.into_iter().enumerate() {
        let Some(y) = hit
            .line
            .checked_sub(top)
            .filter(|y| *y < area.height as usize)
        else {
            continue;
        };
        let style = if Some(i) == current {
            theme::ground("glow")
                .unwrap_or_default()
                .patch(theme::foreground("bg").unwrap_or_default())
        } else {
            theme::ground("glowTint").unwrap_or_default()
        };
        for x in hit.column..(hit.column + hit.width).min(area.width as usize) {
            frame.buffer_mut()[(area.x + x as u16, area.y + y as u16)].set_style(style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn query_edits_reset_selection_and_steps_clamp_at_both_ends() {
        let mut search = Search::open();
        search.type_char('x');
        let hits = hits(&["x x".into()], &search.query);
        search.step(&hits, 1);
        assert_eq!(search.current, Some(1));
        search.step(&hits, -10);
        assert_eq!(search.current, Some(0));
        search.step(&hits, 10);
        assert_eq!(search.current, Some(1));
        search.delete();
        assert_eq!(search.query, "");
        assert_eq!(search.current, None);
        search.step(&[], 1);
        assert_eq!(search.current, None);
    }
    #[test]
    fn matcher_is_case_insensitive_and_counts_a_wide_character_once() {
        assert_eq!(
            hits(&["你好 Reset 你".into()], "你"),
            vec![
                Hit {
                    line: 0,
                    column: 0,
                    width: 2
                },
                Hit {
                    line: 0,
                    column: 11,
                    width: 2
                }
            ]
        );
        assert_eq!(
            hits(&["你好 Reset".into()], "reset"),
            vec![Hit {
                line: 0,
                column: 5,
                width: 5
            }]
        );
        assert!(hits(&["text".into()], "").is_empty());
    }
}
