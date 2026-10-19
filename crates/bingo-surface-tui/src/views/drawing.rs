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

/// What a part's deadline changes when it comes (ADR-0097 §10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Changes {
    /// Its words or its width: it may appear, end, or take other room.
    Size,
    /// Only its colour: nobody sees it where the part has no room.
    Ink,
}

/// A part of a shared row: what it drew, and what its deadline changes.
#[must_use]
#[derive(Debug)]
pub struct Part {
    pub drawing: Drawing,
    pub changes: Changes,
}

impl Part {
    pub fn size(drawing: Drawing) -> Self {
        Self {
            drawing,
            changes: Changes::Size,
        }
    }

    pub fn ink(drawing: Drawing) -> Self {
        Self {
            drawing,
            changes: Changes::Ink,
        }
    }

    /// The deadline that still matters once the row is laid out: a part
    /// clipped to no room keeps only one that may change its size.
    pub fn next(&self, has_room: bool) -> Option<Instant> {
        match (self.changes, has_room) {
            (Changes::Ink, false) => None,
            _ => self.drawing.next,
        }
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
