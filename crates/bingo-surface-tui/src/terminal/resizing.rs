use super::*;
use crate::layout::Canvas;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;

fn screen(area: Rect) -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(area.width, area.height)).unwrap()
}

fn layout(area: Rect, symbol: &str) -> Layout {
    let mut layout = Layout {
        canvas: Canvas::new(area),
        ..Layout::default()
    };
    for cell in &mut layout.canvas.buffer.content {
        cell.set_symbol(symbol);
    }
    layout.canvas.cursor = Some((area.right() - 1, area.bottom() - 1).into());
    layout
}

fn assert_drawn(screen: &mut Terminal<TestBackend>, layout: &Layout) {
    screen.backend().assert_buffer(&layout.canvas.buffer);
    screen
        .backend_mut()
        .assert_cursor_position(layout.canvas.cursor.unwrap());
}

fn resize_between_frames(before: Rect, after: Rect) {
    let mut screen = screen(before);
    let render = Render::default();
    let first = layout(before, "a");
    paint(&mut screen, &first, &render).unwrap();
    assert_drawn(&mut screen, &first);

    // A prebuilt frame can disagree with the size observed by Terminal::draw.
    let next = layout(after, "b");
    paint(&mut screen, &next, &render).unwrap();
    screen
        .backend()
        .assert_buffer(&ratatui::buffer::Buffer::empty(before));
    screen.backend_mut().resize(after.width, after.height);
    paint(&mut screen, &next, &render).unwrap();
    assert_drawn(&mut screen, &next);

    // The backend can also resize after layout but before application.
    let stale = layout(after, "c");
    screen.backend_mut().resize(before.width, before.height);
    paint(&mut screen, &stale, &render).unwrap();
    screen
        .backend()
        .assert_buffer(&ratatui::buffer::Buffer::empty(before));
    let recovered = layout(before, "d");
    paint(&mut screen, &recovered, &render).unwrap();
    assert_drawn(&mut screen, &recovered);
    paint(&mut screen, &recovered, &render).unwrap();
    assert_drawn(&mut screen, &recovered);
}

#[test]
fn paint_survives_wide_to_narrow_resize() {
    resize_between_frames(Rect::new(0, 0, 96, 35), Rect::new(0, 0, 46, 35));
}

#[test]
fn paint_survives_narrow_to_wide_resize() {
    resize_between_frames(Rect::new(0, 0, 46, 35), Rect::new(0, 0, 96, 35));
}

#[test]
fn paint_survives_height_only_resize() {
    resize_between_frames(Rect::new(0, 0, 96, 35), Rect::new(0, 0, 96, 18));
}
