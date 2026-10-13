use ratatui::text::Line;
pub fn lines(columns: u16, rows: u16, cells: &[String], width: usize) -> Vec<Line<'static>> {
    cells
        .iter()
        .take(usize::from(rows))
        .map(|row| super::fit(Line::from(row.clone()), width.min(usize::from(columns))))
        .collect()
}
