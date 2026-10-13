//! Fixed-size layout and paint fixtures.

use super::{Now, SessionState, Tree, Ui, solo};
use crate::layout;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

/// One frame, rendered into a fixed-size buffer, as text.
pub fn render(state: &SessionState, ui: &mut Ui, now: Now) -> String {
    draw_sized(80, 24, state, ui, now)
}

pub fn draw_sized(width: u16, height: u16, state: &SessionState, ui: &mut Ui, now: Now) -> String {
    draw_tree(width, height, &solo(state), ui, now)
}

pub fn render_tree(tree: &Tree, ui: &mut Ui, now: Now) -> String {
    draw_tree(80, 24, tree, ui, now)
}

pub fn draw_tree(width: u16, height: u16, tree: &Tree, ui: &mut Ui, now: Now) -> String {
    drawn(width, height, tree, ui, now).to_string()
}

/// The screen row carrying `needle`, for a test that puts the pointer on the
/// text a person sees rather than on a row counted by hand.
pub fn row_carrying(screen: &str, needle: &str) -> u16 {
    let row = screen
        .lines()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("no row carries {needle:?}:\n{screen}"));
    u16::try_from(row).expect("a row of the screen")
}

/// The terminal one draw leaves, for a test that asks where a style landed.
pub fn drawn(width: u16, height: u16, tree: &Tree, ui: &mut Ui, now: Now) -> TestBackend {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
    ui.prepare(&tree.viewed().summary.cwd);
    let shown = layout::layout(
        std::mem::take(&mut ui.shown),
        tree,
        ui,
        ratatui::layout::Rect::new(0, 0, width, height),
        now,
    );
    terminal
        .draw(|frame| layout::paint(&shown, frame))
        .expect("a drawn frame");
    ui.shown = shown;
    terminal.backend().clone()
}
