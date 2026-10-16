//! A frame's geometry, cells, targets and next change, without a terminal.

use crate::blocks::Blocks;
use crate::clock::Now;
use crate::frame::Regions;
use crate::graphics::Placed;
use crate::tree::Tree;
use crate::ui::{Card, Ui};
use ratatui::layout::Rect;
use std::time::Instant;

mod canvas;
pub(crate) mod count;
mod deadline;
pub use canvas::Canvas;

#[derive(Debug, Default)]
pub struct Layout {
    pub inline: Option<(usize, usize, usize)>,
    pub palette_height: Option<(String, usize)>,
    pub card_actions: Vec<crate::views::targets::Target>,
    pub band_actions: Vec<crate::views::targets::Target>,
    pub seating: crate::pane::seat::Seating,
    pub waiting: Vec<(String, Rect)>,
    pub pulls: Vec<crate::pulled::Key>,
    pub blocks: Blocks,
    /// Header measurements move with the block memo, never through a shared UI borrow.
    pub measurements: crate::graphics::Decoded,
    pub canvas: Canvas,
    pub next_frame: Option<Instant>,
    #[cfg(test)]
    pub renders: [usize; 4],
    pub regions: Regions,
    /// The transcript's height in wrapped lines.
    pub height: usize,
    /// The first transcript line the frame showed.
    pub top: usize,
    /// The card on the screen, when one is open.
    pub card: Option<Card>,
    /// The thumbnails the composer's strip drew, in the line's own order.
    pub strip: Vec<crate::graphics::Picture>,
    /// Where this frame drew each picture's cells, in the terminal's own rows
    /// and columns: what a click on a picture is answered against. It is read
    /// back out of the lines that were drawn
    /// ([`crate::graphics::placed::cells`]), so it can never say a picture is
    /// somewhere the screen does not (M56).
    pub cells: Vec<Placed>,
    /// Where this frame drew the way back to the foot ([`crate::foot`]), when
    /// the transcript was held: what a click there is answered against.
    pub foot: Option<ratatui::layout::Rect>,
    pub crew: Option<ratatui::layout::Rect>,
}

impl Layout {
    /// The picture a pointer is on, when it is on one. The cells under it say
    /// which picture they are cells of, and this frame's own list says which
    /// picture that is — so a click can only ever open a picture the screen is
    /// showing (M56).
    pub fn picture_at(&self, at: ratatui::layout::Position) -> Option<crate::graphics::Picture> {
        let id = self.cells.iter().find(|one| one.area.contains(at))?.id;
        self.placed().into_iter().find(|picture| picture.id() == id)
    }

    /// Every picture this frame put cells on the screen for, oldest first: the
    /// transcript's, then the composer's own. The strip comes last because
    /// what falls off the terminal's cap is the oldest, and the picture a
    /// person is about to send is the newest thing on the screen.
    pub fn placed(&self) -> Vec<crate::graphics::Picture> {
        let mut placed = self.blocks.pictures();
        placed.extend(self.strip.iter().cloned());
        placed
    }

    /// The blank rows a short transcript leaves above itself: it hangs from
    /// the composer, not from the top of the screen, so those rows are nobody's
    /// lines.
    pub fn padding(&self) -> usize {
        let rows = usize::from(self.regions.transcript.height);
        rows - self.height.min(rows)
    }

    /// The transcript line drawn at a row of the transcript region, when that
    /// row carries one. The one mapping from the screen back into the
    /// transcript: a click, a drag and a mark are all answered by it.
    pub fn line_at(&self, row: usize) -> Option<usize> {
        let line = self.top + row.checked_sub(self.padding())?;
        match self.inline {
            Some((at, _, count)) if line >= at && line < at + count => None,
            Some((at, removed, count)) if line >= at + count => Some(line - count + removed),
            _ => Some(line),
        }
    }

    /// The row of the transcript region a line was drawn at, when it is on the
    /// screen — [`Layout::line_at`] the other way round.
    pub fn row_of(&self, line: usize) -> Option<u16> {
        let line = match self.inline {
            Some((at, removed, count)) if line >= at + removed => line + count - removed,
            _ => line,
        };
        let row = u16::try_from(line.checked_sub(self.top)? + self.padding()).ok()?;
        (row < self.regions.transcript.height).then_some(row)
    }
}

impl std::ops::Deref for Layout {
    type Target = Canvas;
    fn deref(&self) -> &Canvas {
        &self.canvas
    }
}
impl std::ops::DerefMut for Layout {
    fn deref_mut(&mut self) -> &mut Canvas {
        &mut self.canvas
    }
}

/// Consume the previous memo; neither the tree nor the UI is changed.
pub fn layout(
    previous: Layout,
    tree: &Tree,
    ui: &Ui,
    size: Rect,
    now: Now,
    render: &crate::render::Render,
) -> Layout {
    render.scope(|| {
        compose(
            previous,
            tree,
            ui,
            size,
            Now {
                motion: render.motion,
                ..now
            },
        )
    })
}

fn compose(previous: Layout, tree: &Tree, ui: &Ui, size: Rect, now: Now) -> Layout {
    let mut next = Layout {
        palette_height: previous
            .palette_height
            .filter(|(id, _)| crate::palette::request(ui) == Some(id.as_str())),
        blocks: previous.blocks,
        measurements: previous.measurements,
        canvas: Canvas::new(size),
        ..Layout::default()
    };
    count::begin();
    crate::view::compose(tree, ui, &mut next, now);
    count::finish(&mut next);
    next.next_frame = deadline::next(tree, ui, &next, now);
    next
}

/// Painting has no geometry, state or clock to decide against.
pub fn paint(layout: &Layout, frame: &mut ratatui::Frame, render: &crate::render::Render) {
    render.scope(|| paint_cells(layout, frame));
}

fn paint_cells(layout: &Layout, frame: &mut ratatui::Frame) {
    // Terminal::draw has resized both buffers. A layout built before that
    // resize must not replace their area; leave a blank frame until rebuilt.
    if frame.area() != layout.canvas.area() {
        return;
    }
    frame.buffer_mut().clone_from(&layout.canvas.buffer);
    if let Some(cursor) = layout.canvas.cursor {
        frame.set_cursor_position(cursor);
    }
}

#[cfg(test)]
mod tests;
