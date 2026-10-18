//! A drawn frame with the styles a snapshot cannot show, and the assertion
//! `docs/design/tui.md` §9 asks for: colour lands only where §4 says it may.
//! A snapshot pins the words; this pins where the eye is sent.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Style;

use crate::clock::Now;
use crate::layout;
use crate::theme::{self, Colors, Theme};
use crate::tree::Tree;
use crate::ui::Ui;

/// Draw the same frame in another look — `NO_COLOR`, or `BINGO_ASCII=1`.
pub fn in_look(theme: Theme, draw: impl FnOnce() -> String) -> String {
    theme::with(theme, draw)
}

pub fn no_colour() -> Theme {
    Theme {
        colors: Colors::Plain,
        glyphs: &theme::UNICODE,
    }
}

pub fn ascii() -> Theme {
    Theme {
        colors: Colors::Ansi,
        glyphs: &theme::ASCII,
    }
}

/// The native look, which is the only one with a ground to raise.
pub fn truecolor() -> Theme {
    Theme {
        colors: Colors::True(theme::DARK),
        glyphs: &theme::UNICODE,
    }
}

/// The same look over a light ground, which is what a terminal that answers
/// `light` to OSC 11 is drawn in.
pub fn daylight() -> Theme {
    Theme {
        colors: Colors::True(theme::LIGHT),
        glyphs: &theme::UNICODE,
    }
}

/// One drawn frame, kept cell by cell.
pub struct Painted(ratatui::buffer::Buffer);

/// Draw a tree and keep the buffer, so a test can ask what colour a row is.
pub fn painted(width: u16, height: u16, tree: &Tree, ui: &mut Ui, now: Now) -> Painted {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
    ui.prepare(&tree.viewed().summary.cwd);
    let shown = layout::layout(
        std::mem::take(&mut ui.shown),
        tree,
        ui,
        ratatui::layout::Rect::new(0, 0, width, height),
        now,
        &crate::render::Render {
            motion: now.motion,
            ..crate::render::current()
        },
    );
    terminal
        .draw(|frame| layout::paint(&shown, frame, &crate::render::current()))
        .expect("a drawn frame");
    ui.shown = shown;
    Painted(terminal.backend().buffer().clone())
}

impl Painted {
    /// The row carrying `needle`, as runs of text that share one style.
    pub fn row(&self, needle: &str) -> Vec<(String, Style)> {
        let area = self.0.area();
        let row = (0..area.height)
            .find(|y| self.text(*y).contains(needle))
            .unwrap_or_else(|| panic!("no row carries {needle:?}:\n{}", self.screen()));
        self.at(row)
    }

    pub fn at(&self, row: u16) -> Vec<(String, Style)> {
        let mut runs: Vec<(String, Style)> = Vec::new();
        for x in 0..self.0.area().width {
            let cell = &self.0[(x, row)];
            match runs.last_mut() {
                Some((text, style)) if *style == cell.style() => text.push_str(cell.symbol()),
                _ => runs.push((cell.symbol().to_string(), cell.style())),
            }
        }
        // Blank cells nothing was spent on are the terminal's padding; blank
        // cells that carry a colour are a bar, and a bar is the view's.
        if let Some((text, style)) = runs.last_mut()
            && !spends_colour(*style)
        {
            *text = text.trim_end().to_string();
        }
        while runs
            .last()
            .is_some_and(|(text, style)| text.is_empty() && !spends_colour(*style))
        {
            runs.pop();
        }
        runs
    }

    /// Every run of that row a colour was spent on.
    pub fn coloured(&self, needle: &str) -> Vec<String> {
        self.row(needle)
            .into_iter()
            .filter(|(text, style)| !text.trim().is_empty() && spends_colour(*style))
            .map(|(text, _)| text.trim().to_string())
            .collect()
    }

    /// Every foreground the frame put in a cell, once each. A picture's own
    /// cells are left out: the kitty protocol carries a picture's id in the
    /// foreground colour (M46), which is a number in a colour's clothing and
    /// not a token at all.
    pub fn inks(&self) -> Vec<ratatui::style::Color> {
        let area = self.0.area();
        let mut out: Vec<ratatui::style::Color> = Vec::new();
        for y in 0..area.height {
            for x in 0..area.width {
                let cell = &self.0[(x, y)];
                if cell
                    .symbol()
                    .starts_with(crate::graphics::kitty::PLACEHOLDER)
                {
                    continue;
                }
                if let Some(ink) = cell.style().fg.filter(|ink| !out.contains(ink)) {
                    out.push(ink);
                }
            }
        }
        out
    }

    fn text(&self, row: u16) -> String {
        (0..self.0.area().width)
            .map(|x| self.0[(x, row)].symbol())
            .collect()
    }

    fn screen(&self) -> String {
        (0..self.0.area().height)
            .map(|y| self.text(y))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Weight is free; hue is what the token table rations.
fn spends_colour(style: Style) -> bool {
    style != theme::as_drawn(Style::new())
        && style != theme::as_drawn(theme::dim())
        && style != theme::as_drawn(theme::bold())
}

/// The row carrying `needle` is exactly these runs, in this order.
pub fn assert_row_styled(painted: &Painted, needle: &str, expected: &[(&str, Style)]) {
    let drawn = painted.row(needle);
    let want: Vec<(String, Style)> = expected
        .iter()
        .map(|(text, style)| ((*text).to_string(), theme::as_drawn(*style)))
        .collect();
    assert_eq!(drawn, want, "the row carrying {needle:?}");
}

#[test]
fn existing_screen_uses_ember_inks_and_respects_terminal_depth() {
    use crate::test_support::*;
    use ratatui::style::Color;
    let state = folded(vec![frame(
        1,
        bingo_sdk::Event::ItemStarted {
            item: user("u1", "hello"),
        },
    )]);
    for look in [
        daylight(),
        no_colour(),
        Theme {
            colors: Colors::Indexed(theme::LIGHT),
            glyphs: &theme::UNICODE,
        },
    ] {
        theme::with(look, || {
            let (mut ui, now) = scene();
            let drawing = painted(100, 32, &solo(&state), &mut ui, now);
            match look.colors {
                Colors::Plain => assert!(
                    drawing
                        .0
                        .content
                        .iter()
                        .all(|cell| cell.fg == Color::Reset && cell.bg == Color::Reset)
                ),
                Colors::Indexed(_) => assert!(drawing.0.content.iter().all(|cell| matches!(
                    cell.fg,
                    Color::Reset | Color::Indexed(_)
                ) && matches!(
                    cell.bg,
                    Color::Reset | Color::Indexed(_)
                ))),
                _ => {
                    assert!(
                        drawing
                            .0
                            .content
                            .iter()
                            .any(|cell| cell.bg == theme::LIGHT.you)
                    );
                    assert!(
                        drawing
                            .0
                            .content
                            .iter()
                            .any(|cell| cell.fg == theme::LIGHT.ember)
                    );
                    assert_eq!(theme::LIGHT.you, Color::Rgb(0xef, 0xea, 0xe8));
                }
            }
        });
    }
}
