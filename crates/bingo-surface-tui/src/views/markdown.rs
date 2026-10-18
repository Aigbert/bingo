//! `View::Markdown`: through the one markdown renderer, so a plugin's prose
//! reads exactly like the model's (design §5).

use ratatui::text::Line;

pub fn lines(text: &str, width: usize, dim: bool) -> Vec<Line<'static>> {
    let mut lines = crate::markdown::render(text, width);
    if dim {
        for line in &mut lines {
            for span in &mut line.spans {
                span.style = span.style.patch(crate::theme::dim());
            }
        }
    }
    lines
}
