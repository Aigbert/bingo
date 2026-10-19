use bingo_services::schedule::Row;
pub(crate) fn selected(rows: &[Row], selection: Option<&str>) -> usize {
    rows.iter()
        .position(|row| Some(row.id.as_str()) == selection)
        .unwrap_or(0)
}
