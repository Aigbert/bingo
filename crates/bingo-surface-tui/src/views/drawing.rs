//! A drawing carries the deadline of the pixels it produced.
use ratatui::text::Line;
use std::time::Instant;

#[must_use]
#[derive(Default, Debug)]
pub struct Drawing {
    pub lines: Vec<Line<'static>>,
    pub next: Option<Instant>,
}

impl Drawing {
    pub fn still(lines: Vec<Line<'static>>) -> Self {
        Self { lines, next: None }
    }

    pub fn map_lines(mut self, map: impl FnOnce(Vec<Line<'static>>) -> Vec<Line<'static>>) -> Self {
        self.lines = map(self.lines);
        self
    }

    pub fn append(&mut self, other: Self) {
        self.next = self.next.into_iter().chain(other.next).min();
        self.lines.extend(other.lines);
    }
}

impl From<Vec<Line<'static>>> for Drawing {
    fn from(lines: Vec<Line<'static>>) -> Self {
        Self::still(lines)
    }
}

impl FromIterator<Drawing> for Drawing {
    fn from_iter<T: IntoIterator<Item = Drawing>>(iter: T) -> Self {
        let mut out = Self::default();
        for drawing in iter {
            out.append(drawing);
        }
        out
    }
}
