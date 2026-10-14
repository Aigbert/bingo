//! A pane's first fitting place, without drawing or state mutation.
use super::Pane;
use bingo_sdk::slot::Place;
use ratatui::layout::Rect;
use std::collections::BTreeMap;

pub const TRANSCRIPT_MIN_COLUMNS: u16 = 60;
pub const TRANSCRIPT_MIN_ROWS: u16 = 8;
pub const FLOATING_MAX: usize = 3;

#[derive(Clone, Debug)]
pub struct Seated {
    pub key: String,
    pub place: Place,
    pub area: Rect,
}
#[derive(Debug, Default)]
pub struct Seating {
    pub area: Rect,
    pub transcript: Rect,
    pub panes: Vec<Seated>,
    pub collapsed: Vec<String>,
}
pub fn seat(area: Rect, wanted: &[Pane], _moved: &BTreeMap<String, Place>) -> Seating {
    Seating { area, transcript: area,
        collapsed: wanted.iter().map(|pane| pane.key.clone()).collect(),
        ..Seating::default() }
}

#[cfg(test)]
mod tests;
