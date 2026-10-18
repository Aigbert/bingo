//! The dock's height and edge, sampled by the frame that draws them.
use crate::{
    clock::{self, Now},
    theme,
    tree::Tree,
    ui::Ui,
    views::Drawing,
};
use ratatui::text::{Line, Span};
use std::time::{Duration, Instant};
mod edge;
pub use edge::draw as edge;

#[derive(Clone, Copy, Debug)]
pub struct Growth {
    from: u16,
    to: u16,
    at: Instant,
}
impl Growth {
    pub fn settled(rows: u16, now: Now) -> Self {
        Self {
            from: rows,
            to: rows,
            at: now.instant,
        }
    }
    pub fn resize(self, rows: u16, now: Now) -> Self {
        if rows == self.to {
            return self;
        }
        Self {
            from: if rows > self.to {
                self.shown(now)
            } else {
                rows
            },
            to: rows,
            at: now.instant,
        }
    }
    pub fn shown(self, now: Now) -> u16 {
        if !now.motion {
            return self.to;
        }
        let length = f32::from(self.to.abs_diff(self.from)) * 0.014;
        let k = if length == 0.0 {
            1.0
        } else {
            now.since(self.at).as_secs_f32() / length
        };
        (f32::from(self.from) + f32::from(self.to - self.from) * clock::ease_out(k)).round() as u16
    }
    pub fn next(self, now: Now) -> Option<Instant> {
        (self.shown(now) < self.to).then(|| now.instant + Duration::from_millis(14))
    }
}

pub fn draw(tree: &Tree, ui: &Ui, width: usize, now: Now) -> Drawing {
    edge(tree, ui, width, now)
}

pub fn ground() -> ratatui::style::Style {
    theme::ground("you").unwrap_or_default()
}

pub fn line(ink: ratatui::style::Style, width: usize) -> Line<'static> {
    Line::from(Span::styled(
        crate::theme::glyphs::get("edge").repeat(width),
        ink,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{later, scene, still};
    #[test]
    fn growth_samples_each_fourteen_ms_step_and_shrinks_at_once() {
        let now = scene().1;
        let growth = Growth::settled(3, now).resize(14, now);
        let heights: Vec<_> = (0..=11).map(|n| growth.shown(later(now, n * 14))).collect();
        assert_eq!(heights, [3, 6, 8, 10, 11, 12, 13, 13, 14, 14, 14, 14]);
        assert_eq!(growth.resize(4, later(now, 28)).shown(later(now, 28)), 4);
        assert_eq!(growth.shown(still(now)), 14);
        assert!(growth.next(later(now, 154)).is_none());
    }
}
