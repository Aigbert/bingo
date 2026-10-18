//! Screen-sized cells composed before the terminal is borrowed.

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::widgets::Widget;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Canvas {
    pub buffer: Buffer,
    pub cursor: Option<Position>,
}

impl Canvas {
    pub fn new(area: Rect) -> Self {
        Self {
            buffer: Buffer::empty(area),
            cursor: None,
        }
    }
    pub fn area(&self) -> Rect {
        self.buffer.area
    }
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }
    pub fn render_widget(&mut self, widget: impl Widget, area: Rect) {
        widget.render(area, &mut self.buffer);
    }
}
