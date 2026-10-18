//! Collect a part's wake before its content is clipped or placed.
use std::time::Instant;

pub trait Part {
    type Content;
    fn split(self) -> (Self::Content, Option<Instant>);
}

impl Part for crate::views::Drawing {
    type Content = Vec<ratatui::text::Line<'static>>;
    fn split(self) -> (Self::Content, Option<Instant>) {
        (self.lines, self.next)
    }
}

impl Part for crate::component::sealed::Drawing {
    type Content = Self;
    fn split(mut self) -> (Self::Content, Option<Instant>) {
        let next = self.next.take();
        (self, next)
    }
}
