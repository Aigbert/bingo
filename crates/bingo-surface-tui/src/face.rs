//! A name's stable visual identity.
use ratatui::style::Style;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Face {
    pub hash: u32,
    pub hue: usize,
    pub sigil: &'static str,
}

pub fn face(_name: &str) -> Face {
    Face {
        hash: 0,
        hue: 0,
        sigil: "◆",
    }
}

pub fn style(_name: &str) -> Style {
    crate::theme::text()
}
